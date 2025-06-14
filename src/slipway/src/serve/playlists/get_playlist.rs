use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{HttpRequest, get, web};
use serde::Deserialize;
use tracing::{Instrument, debug_span, info_span};

use crate::primitives::{DeviceName, PlaylistName};
use crate::serve::repository::RigResultSpec;
use crate::serve::responses::{FormatQuery, PlaylistResponse, ServeError};
use crate::serve::rigs::get_rig::RequestingDevice;

use super::super::ServeState;

#[derive(Deserialize)]
struct GetPlaylistPath {
    playlist_name: PlaylistName,
}

#[derive(Deserialize)]
struct GetPlaylistQuery {
    #[serde(flatten)]
    output: FormatQuery,

    device: Option<DeviceName>,
}

#[get("/playlists/{playlist_name}")]
pub async fn get_playlist(
    path: web::Path<GetPlaylistPath>,
    query: web::Query<GetPlaylistQuery>,
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<PlaylistResponse, ServeError> {
    let path = path.into_inner();
    let query = query.into_inner();
    let state = data.into_inner();

    let playlist_name = &path.playlist_name;
    let result_spec = query.output.into_spec();

    let device = match query.device {
        Some(device_name) => {
            let device = state.repository.get_device(&device_name).await?;
            Some(RequestingDevice::from(device_name, device))
        }
        None => None,
    };

    get_playlist_response(playlist_name, device, result_spec, state, req)
        .instrument(info_span!("playlist", ""=%playlist_name))
        .await
}

pub async fn get_playlist_response(
    playlist_name: &PlaylistName,
    device: Option<RequestingDevice>,
    result_spec: RigResultSpec,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<PlaylistResponse, ServeError> {
    let maybe_playlist_item =
        super::evaluate_playlist::evaluate_playlist(Arc::clone(&state), playlist_name).await?;

    let Some(playlist_item) = maybe_playlist_item else {
        return Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            "Playlist item not found for the current day and time.".to_string(),
        ));
    };

    let rig_name = playlist_item.rig;
    let refresh_rate_seconds = playlist_item.refresh_rate_seconds;

    let rig_response =
        super::super::rigs::get_rig::get_rig_response(&rig_name, device, result_spec, state, req)
            .instrument(debug_span!("rig", ""=%rig_name))
            .await?;

    Ok(PlaylistResponse {
        refresh_rate_seconds,
        rig_response,
    })
}
