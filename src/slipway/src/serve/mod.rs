use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::middleware::{from_fn, Next};
use actix_web::{web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use repository::ServeRepository;
use serde::Deserialize;
use slipway_engine::Permission;

use base64::prelude::*;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use thiserror::Error;
use tracing::{info, warn};

pub(super) mod commands;
mod get_rig;
mod repository;
mod run_rig;
pub(super) mod trmnl;
use sha2::{Digest, Sha256};

use crate::permissions::PermissionsOwned;
use crate::primitives::RigName;

fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}

struct ServeState {
    pub root: PathBuf,
    pub config: SlipwayServeConfig,
    pub expected_authorization_header: Option<String>,
    pub repository: Box<dyn ServeRepository>,
}

impl ServeState {
    pub fn new(
        root: PathBuf,
        config: SlipwayServeConfig,
        expected_authorization_header: Option<String>,
        repository: Box<dyn ServeRepository>,
    ) -> Self {
        Self {
            root,
            config,
            expected_authorization_header,
            repository,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
struct SlipwayServeConfig {
    #[serde(default)]
    log_level: Option<String>,

    #[serde(default)]
    registry_urls: Vec<String>,

    #[serde(default)]
    rig_permissions: HashMap<RigName, PermissionsOwned>,

    #[serde(default)]
    repository: RepositoryConfig,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
enum RepositoryConfig {
    #[default]
    ReadOnlyFilesystem,
}

pub async fn serve(path: PathBuf) -> anyhow::Result<()> {
    let config = load_serve_config(&path).await?;
    serve_with_config(path, config).await?;
    Ok(())
}

async fn load_serve_config(root_path: &Path) -> Result<SlipwayServeConfig, anyhow::Error> {
    let config_path = root_path.join("slipway_serve.json");
    let config = match tokio::fs::read(&config_path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).context("Failed to parse Slipway Serve config file.")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SlipwayServeConfig::default(),
        Err(e) => return Err(e).context("Failed to load Slipway Serve config file.")?,
    };
    Ok(config)
}

fn create_repository(root_path: &Path, config: &RepositoryConfig) -> Box<dyn ServeRepository> {
    match config {
        RepositoryConfig::ReadOnlyFilesystem => Box::new(
            repository::file_system::FileSystemRepository::new(root_path.to_owned()),
        ),
    }
}

async fn serve_with_config(root: PathBuf, config: SlipwayServeConfig) -> anyhow::Result<()> {
    super::configure_tracing(config.log_level.clone());

    let expected_authorization_header = std::env::var("SLIPWAY_AUTHORIZATION_HEADER").ok();

    info!("Starting Slipway Serve with config: {:?}", config);

    if expected_authorization_header.is_some() {
        info!("Authorization header required for all requests.");
    } else {
        warn!("No authorization header required for requests.");
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(ServeState::new(
                root.clone(),
                config.clone(),
                expected_authorization_header.clone(),
                create_repository(&root, &config.repository),
            )))
            .wrap(from_fn(auth_middleware))
            .service(get_rig::get_rig)
            .service(trmnl::trmnl_setup)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

#[derive(Clone)]
struct RequestState {
    pub authorized_header: Option<String>,
}

async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let serve_state = req
        .app_data::<web::Data<ServeState>>()
        .expect("ServeState should exist.");

    if let Some(expected_authorization_header) = serve_state.expected_authorization_header.as_ref()
    {
        let actual_authorization_header = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().map(Cow::Borrowed).ok())
            .unwrap_or_else(|| {
                let query_string = req.query_string();
                // convert query string into a map
                let query_map: std::collections::HashMap<_, _> =
                    url::form_urlencoded::parse(query_string.as_bytes()).collect();

                let query_auth = query_map.get("authorization");

                query_auth.cloned().unwrap_or(Cow::Borrowed(""))
            });

        if actual_authorization_header.as_ref() != expected_authorization_header.as_str() {
            return Err(ServeError::UserFacing(
                StatusCode::UNAUTHORIZED,
                "Unauthorized".to_string(),
            )
            .into());
        }

        req.extensions_mut().insert(RequestState {
            authorized_header: Some(actual_authorization_header.into_owned()),
        });
    } else {
        req.extensions_mut().insert(RequestState {
            authorized_header: None,
        });
    }

    next.call(req).await
}

#[derive(Debug, Error)]
enum ServeError {
    #[error("internal error: {0}")]
    Internal(anyhow::Error),

    #[error("{1}")]
    UserFacing(StatusCode, String),
}

impl actix_web::error::ResponseError for ServeError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ServeError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServeError::UserFacing(status_code, _) => status_code,
        }
    }
}

enum RigResponse {
    Image(ImageResponse),
    Json(web::Json<serde_json::Value>),
    Html(String),
}

impl Responder for RigResponse {
    type Body = EitherBody<std::string::String>;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RigResponse::Image(image) => image.respond_to(_req).map_into_right_body(),
            RigResponse::Json(json) => json.respond_to(_req),
            RigResponse::Html(html) => HttpResponse::Ok()
                .content_type(ContentType::html())
                .body(html)
                .map_into_right_body(),
        }
    }
}

struct ImageResponse {
    image: RgbaImage,
    format: ImageFormat,
    wrap_in_html: bool,
}

// Responder
impl Responder for ImageResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let width = self.image.width();

        // Convert your RgbaImage to a DynamicImage.
        let dynamic = DynamicImage::ImageRgba8(self.image);

        // Create a buffer to hold the output bytes.
        let mut buf = Cursor::new(Vec::new());

        // Write the image as PNG into the buffer.
        dynamic
            .write_to(&mut buf, self.format)
            .expect("Failed to encode image.");

        // Extract the raw PNG bytes.
        let image_bytes = buf.into_inner();

        if self.wrap_in_html {
            let html = format!(
                r#"<html><head><meta name="viewport" content="width={width}"></head><body style="margin:0px; width={width}px"><img src="data:image/png;base64,{}"/></body></html>"#,
                BASE64_STANDARD.encode(&image_bytes)
            );

            return HttpResponse::Ok()
                .content_type(ContentType::html())
                .body(html);
        }

        let body = image_bytes;

        // Create response and set content type
        let mut response = HttpResponse::Ok();

        match self.format {
            ImageFormat::Jpeg => {
                response.content_type(ContentType::jpeg());
            }
            ImageFormat::Png => {
                response.content_type(ContentType::png());
            }
            _ => {}
        };

        response.body(body)
    }
}
