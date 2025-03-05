use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest};
use serde::Deserialize;
use tracing::{debug_span, info_span, Instrument};

use crate::primitives::DeviceName;
use crate::serve::{PlaylistResponse, RigResultImageFormat, RigResultPresentation};

use super::super::{ServeError, ServeState};

#[derive(Deserialize)]
struct GetDevicePath {
    device_name: DeviceName,
}

#[derive(Deserialize)]
struct GetDeviceQuery {
    #[serde(default)]
    format: Option<RigResultImageFormat>,

    #[serde(default)]
    presentation: Option<RigResultPresentation>,
}

#[get("/device/{device_name}")]
pub async fn get_device(
    path: web::Path<GetDevicePath>,
    query: web::Query<GetDeviceQuery>,
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<PlaylistResponse, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();
    let state = data.into_inner();

    let device_name = &path.device_name;
    let format = query.format.unwrap_or_default();
    let presentation = query.presentation.unwrap_or_default();

    get_device_response(device_name, format, presentation, state, req)
        .instrument(info_span!("device", %device_name))
        .await
}

pub async fn get_device_response(
    device_name: &DeviceName,
    format: RigResultImageFormat,
    presentation: RigResultPresentation,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<PlaylistResponse, ServeError> {
    let device = state.repository.get_device(device_name).await?;

    let maybe_playlist_name = device.playlist.as_ref();

    let Some(playlist_name) = maybe_playlist_name else {
        return Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            "Device has no playlist.".to_string(),
        ));
    };

    super::super::playlists::get_playlist::get_playlist_response(
        playlist_name,
        format,
        presentation,
        state,
        req,
    )
    .instrument(debug_span!("playlist", %playlist_name))
    .await
}
