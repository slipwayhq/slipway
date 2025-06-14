use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{HttpRequest, get, web};
use serde::Deserialize;
use tracing::{Instrument, debug_span, info_span};

use crate::primitives::DeviceName;
use crate::serve::repository::RigResultSpec;
use crate::serve::responses::{FormatQuery, PlaylistResponse};
use crate::serve::rigs::get_rig::RequestingDevice;

use super::super::{ServeState, responses::ServeError};

#[derive(Deserialize)]
struct GetDevicePath {
    device_name: DeviceName,
}

#[derive(Deserialize)]
struct GetDeviceQuery {
    #[serde(flatten)]
    output: FormatQuery,
}

#[get("/devices/{device_name}")]
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

    get_device_response(device_name, query.output, None, state, req)
        .instrument(info_span!("device", ""=%device_name))
        .await
}

pub async fn get_device_response(
    device_name: &DeviceName,
    format_query: FormatQuery,
    default_result_spec: Option<RigResultSpec>,
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

    let result_spec_defaults = device
        .result_spec
        .into_spec(default_result_spec.unwrap_or_default());
    let result_spec = format_query.into_spec_with_defaults(result_spec_defaults);

    super::super::playlists::get_playlist::get_playlist_response(
        playlist_name,
        Some(RequestingDevice {
            name: device_name.clone(),
            context: device.context,
        }),
        result_spec,
        state,
        req,
    )
    .instrument(debug_span!("playlist", ""=%playlist_name))
    .await
}
