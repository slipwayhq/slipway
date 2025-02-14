use std::path::PathBuf;
use std::sync::Arc;

use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{get, web, App, Either, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use serde::Deserialize;
use slipway_engine::Permission;

use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use tracing::info;
mod rig;
use thiserror::Error;

#[derive(Clone)]
struct ServeState {
    pub root: PathBuf,
    pub config: SlipwayServeConfig,
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

    info!("Starting Slipway Serve with config: {:?}", config);

    let state = web::Data::new(ServeState { root, config });

    HttpServer::new(move || App::new().app_data(state.clone()).service(get_rig))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await?;

    Ok(())
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

struct ImageResponse {
    image: RgbaImage,
    format: ImageFormat,
}

// Responder
impl Responder for ImageResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
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
    result_type: Option<RigResultType>,
}

#[derive(Deserialize)]
enum RigResultType {
    Jpeg,
    Png,
    Json,
}

#[get("/rig/{rig_name}")]
async fn get_rig(
    path: web::Path<GetRigPath>,
    query: web::Query<GetRigQuery>,
    data: web::Data<ServeState>,
) -> Result<Either<web::Json<serde_json::Value>, ImageResponse>, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();
    let state = data.into_inner();
    get_rig_inner(path.rig_name, query.result_type, state).await
}

async fn get_rig_inner(
    rig_name: String,
    result_type: Option<RigResultType>,
    state: Arc<ServeState>,
) -> Result<Either<web::Json<serde_json::Value>, ImageResponse>, ServeError> {
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

    match result_type {
        None | Some(RigResultType::Png) | Some(RigResultType::Jpeg) => {
            let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

            if let Ok(image) = maybe_image {
                Ok(Either::Right(ImageResponse {
                    image,
                    format: match result_type {
                        Some(RigResultType::Jpeg) => ImageFormat::Jpeg,
                        _ => ImageFormat::Png,
                    },
                }))
            } else {
                match result_type {
                    None => Ok(Either::Left(web::Json(result.output))),
                    _ => Err(ServeError::UserFacing(
                        StatusCode::BAD_REQUEST,
                        "Could not render first rig output as an image.".to_string(),
                    )),
                }
            }
        }
        Some(RigResultType::Json) => Ok(Either::Left(web::Json(result.output))),
    }
}
