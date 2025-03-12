use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use actix_cors::Cors;
use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::middleware::{from_fn, Next, NormalizePath, TrailingSlash};
use actix_web::{web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use chrono_tz::Tz;
use repository::{Device, Playlist, ServeRepository};
use serde::{Deserialize, Serialize};

use base64::prelude::*;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use thiserror::Error;
use tracing::{debug, info, warn};
use url::Url;

#[cfg(test)]
mod api_tests;
mod bmp;
pub(super) mod commands;
mod devices;
mod playlists;
mod repository;
mod rigs;
pub(super) mod trmnl;

use sha2::{Digest, Sha256};

use crate::permissions::PermissionsOwned;
use crate::primitives::{DeviceName, PlaylistName, RigName};

const REFRESH_RATE_HEADER: &str = "refresh-rate";
const ACCESS_TOKEN_HEADER: &str = "access-token";
const ID_HEADER: &str = "id";

const TRMNL_PATH: &str = "/api";
const TRMNL_DISPLAY_PATH: &str = "/api/display";
const API_GET_RIG_PATH: &str = "/rig";

fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}

fn hash_bytes(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    format!("{:x}", result)
}

fn create_friendly_id(hashed_api_key: &str) -> String {
    hashed_api_key[..6].to_string()
}

fn create_api_key() -> String {
    nanoid::nanoid!(64)
}

#[derive(Debug)]
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
#[serde(deny_unknown_fields)]
struct SlipwayServeConfig {
    #[serde(default)]
    log_level: Option<String>,

    #[serde(default)]
    registry_urls: Vec<String>,

    #[serde(default)]
    timezone: Option<Tz>,

    #[serde(default)]
    rig_permissions: HashMap<RigName, PermissionsOwned>,

    #[serde(default)]
    repository: RepositoryConfig,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
enum RepositoryConfig {
    #[default]
    Filesystem,
    Memory {
        devices: HashMap<DeviceName, Device>,
        playlists: HashMap<PlaylistName, Playlist>,
        rigs: HashMap<RigName, slipway_engine::Rig>,
    },
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
        RepositoryConfig::Filesystem => Box::new(
            repository::file_system::FileSystemRepository::new(root_path.to_owned()),
        ),
        RepositoryConfig::Memory {
            devices,
            playlists,
            rigs,
        } => Box::new(repository::memory::MemoryRepository::new(
            devices.clone(),
            playlists.clone(),
            rigs.clone(),
        )),
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
        create_app(
            root.clone(),
            config.clone(),
            expected_authorization_header.clone(),
        )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

fn create_app(
    root: PathBuf,
    config: SlipwayServeConfig,
    expected_authorization_header: Option<String>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let repository = create_repository(&root, &config.repository);

    App::new()
        .app_data(web::Data::new(ServeState::new(
            root,
            config,
            expected_authorization_header,
            repository,
        )))
        .wrap(
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header(),
        )
        .service(
            // Trmnl services.
            web::scope(TRMNL_PATH)
                .wrap(NormalizePath::new(TrailingSlash::Trim)) // Required for TRMNL device as of 2025-03-07.
                .wrap(from_fn(trmnl_auth_middleware))
                .service(trmnl::trmnl_setup)
                .service(trmnl::trmnl_display)
                .service(trmnl::trmnl_log),
        )
        .service(
            // Non-Trmnl services.
            web::scope("")
                .wrap(from_fn(auth_middleware))
                .service(rigs::get_rig::get_rig)
                .service(playlists::get_playlist::get_playlist)
                .service(devices::get_device::get_device),
        )
}

#[derive(Clone)]
struct RequestState {
    pub authorized_header: Option<String>,
    pub required_authorization_header: Option<String>,
}

/// Non-Trmnl endpoints use an optional Authorization header for authentication.
async fn auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    debug!("Running auth_middleware for {}", req.request().path());

    let serve_state = req
        .app_data::<web::Data<ServeState>>()
        .expect("ServeState should exist.");

    if let Some(expected_authorization_header) = serve_state.expected_authorization_header.as_ref()
    {
        let actual_authorization_header = req
            .headers()
            .get("authorization")
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
            required_authorization_header: Some(expected_authorization_header.clone()),
        });
    } else {
        req.extensions_mut().insert(RequestState {
            authorized_header: None,
            required_authorization_header: None,
        });
    }

    next.call(req).await
}

/// Trmnl endpoints use an API key for authentication, but when we return a URL to the
/// device we need to include the API key in the URL. This middleware ensures that the
/// API key populated in the RequestState if it is required.
async fn trmnl_auth_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    debug!("Running trmnl_auth_middleware for {}", req.request().path());

    let serve_state = req
        .app_data::<web::Data<ServeState>>()
        .expect("ServeState should exist.");

    req.extensions_mut().insert(RequestState {
        authorized_header: None,
        required_authorization_header: serve_state.expected_authorization_header.clone(),
    });

    next.call(req).await
}

#[derive(Debug, Error)]
enum ServeError {
    #[error("internal error: {0}")]
    Internal(anyhow::Error),

    #[error("{0}: {1}")]
    UserFacing(StatusCode, String),

    #[error("{0}: {1}")]
    UserFacingJson(StatusCode, serde_json::Value),
}

impl actix_web::error::ResponseError for ServeError {
    fn error_response(&self) -> HttpResponse {
        debug!("Error response: {:?}", self);

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ServeError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServeError::UserFacing(status_code, _) => status_code,
            ServeError::UserFacingJson(status_code, _) => status_code,
        }
    }
}

enum RigResponse {
    Image(ImageResponse),
    Json(web::Json<serde_json::Value>),
    Url(UrlResponse),
}

impl Responder for RigResponse {
    type Body = EitherBody<std::string::String>;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            RigResponse::Image(image) => image.respond_to(req).map_into_right_body(),
            RigResponse::Json(json) => json.respond_to(req),
            RigResponse::Url(url) => url.respond_to(req).map_into_right_body(),
        }
    }
}

struct PlaylistResponse {
    refresh_rate_seconds: u32,
    rig_response: RigResponse,
}

impl Responder for PlaylistResponse {
    type Body = EitherBody<std::string::String>;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let mut response = match self.rig_response {
            RigResponse::Image(image) => image.respond_to(req).map_into_right_body(),
            RigResponse::Json(json) => json.respond_to(req),
            RigResponse::Url(url) => url.respond_to(req).map_into_right_body(),
        };

        response.headers_mut().append(
            HeaderName::from_static(REFRESH_RATE_HEADER),
            HeaderValue::from_str(&self.refresh_rate_seconds.to_string())
                .expect("Refresh rate header value should be valid."),
        );

        response
    }
}

struct UrlResponse {
    url: Url,
}

impl Responder for UrlResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let url = self.url;
        let html = format!(
            r#"<html><body style="margin:0px"><img src="{}"/></body></html>"#,
            url
        );

        HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(html)
    }
}

struct ImageResponse {
    image: RgbaImage,
    format: RigResultImageFormat,
    wrap_in_html: bool,
}

impl Responder for ImageResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let width = self.image.width();

        let maybe_image_bytes = match self.format {
            RigResultImageFormat::Jpeg => get_image_bytes(self.image, ImageFormat::Jpeg),
            RigResultImageFormat::Png => get_image_bytes(self.image, ImageFormat::Png),
            RigResultImageFormat::Bmp1Bit => bmp::encode_1bit_bmp(self.image),
        };

        let image_bytes = match maybe_image_bytes {
            Err(e) => {
                return HttpResponse::InternalServerError().body(format!("{:?}", e));
            }
            Ok(image_bytes) => image_bytes,
        };

        info!("Responding with image of size {} bytes.", image_bytes.len());

        if self.wrap_in_html {
            let html = format!(
                r#"<html><head><meta name="viewport" content="width={width}"></head><body style="margin:0px; width={width}px"><img src="data:image/png;base64,{}"/></body></html>"#,
                BASE64_STANDARD.encode(&image_bytes)
            );

            return HttpResponse::Ok()
                .content_type(ContentType::html())
                .body(html);
        }

        let maybe_etag = _req
            .headers()
            .get("if-none-match")
            .and_then(|v| v.to_str().ok());

        if let Some(etag) = maybe_etag {
            debug!("Device supplied ETag: {}", etag);
            let new_etag = hash_bytes(&image_bytes);
            debug!("Calculated ETag     : {}", new_etag);

            if etag == new_etag {
                info!("Returning 304 Not Modified.");
                return HttpResponse::NotModified().finish();
            }
        }

        let etag = hash_bytes(&image_bytes);

        let body = image_bytes;

        let mut response = HttpResponse::Ok();

        response.insert_header((
            HeaderName::from_static("etag"),
            HeaderValue::from_str(&etag).expect("ETag value should lowercase hex"),
        ));

        match self.format {
            RigResultImageFormat::Jpeg => {
                response.content_type(ContentType::jpeg());
            }
            RigResultImageFormat::Png => {
                response.content_type(ContentType::png());
            }
            RigResultImageFormat::Bmp1Bit => {
                response.content_type("image/bmp");
            }
        };

        response.body(body)
    }
}

fn get_image_bytes(image: RgbaImage, format: ImageFormat) -> Result<Vec<u8>, image::ImageError> {
    let dynamic = DynamicImage::ImageRgba8(image);

    let mut buf = Cursor::new(Vec::new());

    dynamic.write_to(&mut buf, format)?;

    Ok(buf.into_inner())
}

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
enum RigResultImageFormat {
    Jpeg,

    #[default]
    Png,

    // Specify serde string as "bmp_1bit", to avoid default of `bmp1_bit`.
    #[serde(rename = "bmp_1bit")]
    Bmp1Bit,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
enum RigResultFormat {
    /// Return an image.
    #[default]
    Image,

    /// Return the JSON output of the rig.
    Json,

    /// Return the image encoded as a data URL.
    DataUrl,

    /// Return a URL which will generate the image.
    Url,
}

#[derive(Deserialize)]
struct FormatQuery {
    #[serde(default)]
    image_format: Option<RigResultImageFormat>,

    #[serde(default)]
    format: Option<RigResultFormat>,
}
