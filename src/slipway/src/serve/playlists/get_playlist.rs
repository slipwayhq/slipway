use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest};
use serde::Deserialize;

use crate::primitives::PlaylistName;
use crate::serve::{PlaylistResponse, RigResultImageFormat, RigResultPresentation};

use super::super::{ServeError, ServeState};

#[derive(Deserialize)]
struct GetPlaylistPath {
    playlist_name: PlaylistName,
}

#[derive(Deserialize)]
struct GetPlaylistQuery {
    #[serde(default)]
    format: Option<RigResultImageFormat>,

    #[serde(default)]
    presentation: Option<RigResultPresentation>,
}

#[get("/playlist/{playlist_name}")]
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
    let format = query.format.unwrap_or_default();
    let presentation = query.presentation.unwrap_or_default();

    get_playlist_response(playlist_name, format, presentation, state, req).await
}

pub async fn get_playlist_response(
    playlist_name: &PlaylistName,
    format: RigResultImageFormat,
    presentation: RigResultPresentation,
    state: Arc<ServeState>,
    req: HttpRequest,
) -> Result<PlaylistResponse, ServeError> {
    let maybe_playlist_item =
        super::evaluate_playlist::evaluate_playlist(Arc::clone(&state), playlist_name)
            .await
            .map_err(ServeError::Internal)?;

    let Some(playlist_item) = maybe_playlist_item else {
        return Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            "Playlist item not found for the current day and time.".to_string(),
        ));
    };

    let rig_name = playlist_item.rig;
    let refresh_rate_seconds = playlist_item.refresh_rate_seconds;

    let rig_response =
        super::super::rigs::get_rig::get_rig_response(&rig_name, format, presentation, state, req)
            .await?;

    Ok(PlaylistResponse {
        refresh_rate_seconds,
        rig_response,
    })
}
