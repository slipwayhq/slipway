use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::middleware::{from_fn, Next};
use actix_web::{get, web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use serde::Deserialize;
use slipway_engine::Permission;

use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use tracing::{info, warn};
mod rig;
use base64::prelude::*;
use thiserror::Error;

#[derive(Clone)]
struct ServeState {
    pub root: PathBuf,
    pub config: SlipwayServeConfig,
    pub expected_authorization_header: Option<String>,
}

impl ServeState {
    pub fn new(
        root: PathBuf,
        config: SlipwayServeConfig,
        expected_authorization_header: Option<String>,
    ) -> Self {
        Self {
            root,
            config,
            expected_authorization_header,
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, Clone)]
struct SlipwayServeConfig {
    #[serde(default)]
    log_level: Option<String>,

    #[serde(default)]
    registry_urls: Vec<String>,

    #[serde(default)]
    allow: Vec<Permission>,

    #[serde(default)]
    deny: Vec<Permission>,
}

pub async fn serve(path: PathBuf) -> anyhow::Result<()> {
    let root = path.to_owned();

    let config_path = path.join("slipway_serve.json");

    let config = match std::fs::File::open(&config_path) {
        Ok(file) => {
            serde_json::from_reader(file).context("Failed to parse Slipway Serve config file.")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SlipwayServeConfig::default(),
        Err(e) => return Err(e).context("Failed to load Slipway Serve config file.")?,
    };

    serve_config(root, config).await?;

    Ok(())
}

async fn serve_config(root: PathBuf, config: SlipwayServeConfig) -> anyhow::Result<()> {
    super::configure_tracing(config.log_level.clone());

    let expected_authorization_header = std::env::var("SLIPWAY_AUTHORIZATION_HEADER").ok();

    info!("Starting Slipway Serve with config: {:?}", config);

    if expected_authorization_header.is_some() {
        info!("Authorization header required for all requests.");
    } else {
        warn!("No authorization header required for requests.");
    }

    let state = web::Data::new(ServeState::new(root, config, expected_authorization_header));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(from_fn(auth_middleware))
            .service(get_rig)
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
    InternalError(anyhow::Error),

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
            ServeError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

#[derive(Deserialize)]
struct GetRigPath {
    rig_name: String,
}

#[derive(Deserialize)]
struct GetRigQuery {
    #[serde(default)]
    format: Option<RigResultFormat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RigResultFormat {
    Jpeg,
    Png,
    Json,
    PngHtml,
    JpegHtml,
    PngHtmlNoEmbed,
    JpegHtmlNoEmbed,
}

#[get("/rig/{rig_name}")]
async fn get_rig(
    path: web::Path<GetRigPath>,
    query: web::Query<GetRigQuery>,
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<RigResponse, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();
    let state = data.into_inner();

    match query.format {
        Some(RigResultFormat::PngHtmlNoEmbed) | Some(RigResultFormat::JpegHtmlNoEmbed) => {
            let connection_info = req.connection_info();
            let scheme = connection_info.scheme();
            let host = connection_info.host();
            let uri = req.uri();
            let path = uri.path();

            let full_url = format!("{}://{}{}", scheme, host, path);

            let mut qs = url::form_urlencoded::Serializer::new(String::new());

            qs.append_pair(
                "format",
                match query.format {
                    Some(RigResultFormat::JpegHtmlNoEmbed) => "jpeg",
                    _ => "png",
                },
            );

            if let Some(authorization) = req
                .extensions()
                .get::<RequestState>()
                .and_then(|state| state.authorized_header.as_ref())
            {
                qs.append_pair("authorization", authorization);
            }

            Ok(RigResponse::Html(format!(
                r#"<html><body style="margin:0px"><img src="{}?{}"/></body></html>"#,
                full_url,
                qs.finish()
            )))
        }
        _ => get_rig_inner(path.rig_name, query.format, state).await,
    }
}

async fn get_rig_inner(
    rig_name: String,
    result_format: Option<RigResultFormat>,
    state: Arc<ServeState>,
) -> Result<RigResponse, ServeError> {
    let rig_path = state.root.join(format!("{rig_name}.json"));
    let rig_json = match std::fs::File::open(&rig_path) {
        Ok(file) => serde_json::from_reader(file)
            .context(format!(
                "Failed to parse Slipway Rig \"{:?}\" as JSON.",
                rig_path
            ))
            .map_err(ServeError::InternalError)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ServeError::UserFacing(
                StatusCode::NOT_FOUND,
                format!("Failed to find Slipway Rig {:?}.", rig_path),
            ))
        }
        Err(e) => return Err(ServeError::InternalError(e.into())),
    };

    let result = rig::run_rig(state, &rig_name, rig_json)
        .await
        .map_err(ServeError::InternalError)?;

    match result_format {
        None
        | Some(RigResultFormat::Png)
        | Some(RigResultFormat::Jpeg)
        | Some(RigResultFormat::PngHtml)
        | Some(RigResultFormat::JpegHtml) => {
            let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

            if let Ok(image) = maybe_image {
                Ok(RigResponse::Image(ImageResponse {
                    image,
                    format: match result_format {
                        Some(RigResultFormat::Jpeg) => ImageFormat::Jpeg,
                        _ => ImageFormat::Png,
                    },
                    wrap_in_html: matches!(
                        result_format,
                        Some(RigResultFormat::PngHtml) | Some(RigResultFormat::JpegHtml)
                    ),
                }))
            } else {
                match result_format {
                    None => Ok(RigResponse::Json(web::Json(result.output))),
                    _ => Err(ServeError::UserFacing(
                        StatusCode::BAD_REQUEST,
                        "Could not render first rig output as an image.".to_string(),
                    )),
                }
            }
        }
        Some(RigResultFormat::Json) => Ok(RigResponse::Json(web::Json(result.output))),
        Some(RigResultFormat::JpegHtmlNoEmbed) | Some(RigResultFormat::PngHtmlNoEmbed) => {
            unreachable!();
        }
    }
}
