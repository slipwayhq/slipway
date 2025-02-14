use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::State;
use axum::response::Response;
use slipway_engine::Permission;

use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::{extract::Path, response::IntoResponse, routing::get, Router};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

mod rig;

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

    // build our application with a route
    let app = Router::new()
        .route("/rig/png/:rig_name", get(get_rig_png))
        .with_state(Arc::new(ServeState { root, config }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_rig_png(
    Path(rig_name): Path<String>,
    State(state): State<Arc<ServeState>>,
) -> Response {
    let result = get_rig_png_inner(rig_name, state).await;
    match result {
        Ok(response) => response.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_rig_png_inner(
    rig_name: String,
    state: Arc<ServeState>,
) -> Result<impl IntoResponse, AppError> {
    let rig_path = state.root.join(format!("{rig_name}.json"));
    let rig_json = match std::fs::File::open(&rig_path) {
        Ok(file) => serde_json::from_reader(file).context(format!(
            "Failed to parse Slipway Rig \"{:?}\" as JSON.",
            rig_path
        ))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                StatusCode::NOT_FOUND,
                format!("Failed to find Slipway Rig \"{:?}\".", rig_path),
            )
                .into_response())
        }
        Err(e) => return Err(AppError(e.into())),
    };

    let result = rig::run_rig(state, &rig_name, rig_json).await?;

    let maybe_image = crate::canvas::get_canvas_image(&result.handle, &result.output);

    if let Ok(image) = maybe_image {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("image/png"));

        // Convert your RgbaImage to a DynamicImage.
        let dynamic = DynamicImage::ImageRgba8(image);

        // Create a buffer to hold the PNG bytes.
        let mut buf = Cursor::new(Vec::new());

        // Write the image as PNG into the buffer.
        dynamic
            .write_to(&mut buf, ImageFormat::Png)
            .expect("Failed to encode image as PNG");

        // Extract the raw PNG bytes.
        let png_bytes = buf.into_inner();

        return Ok((headers, png_bytes).into_response());
    } else {
        return Ok((
            StatusCode::BAD_REQUEST,
            "Could not render first rig output as an image.",
        )
            .into_response());
    }
}

struct ServeState {
    pub root: PathBuf,
    pub config: SlipwayServeConfig,
}

#[derive(Debug, Default, serde::Deserialize)]
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

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
