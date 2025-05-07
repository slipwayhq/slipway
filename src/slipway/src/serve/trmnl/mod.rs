mod display;
mod log;
mod setup;

use actix_web::{
    HttpRequest,
    http::{StatusCode, header::HeaderMap},
};
pub(super) use display::trmnl_display;
pub(super) use log::trmnl_log;
pub(super) use setup::trmnl_setup;
use slipway_host::hash_string;
use tracing::{debug, info};

use crate::{primitives::DeviceName, serve::write_api_key_message};

use super::{
    ACCESS_TOKEN_HEADER, ID_HEADER, ShowApiKeys,
    repository::{Device, TrmnlDevice},
    responses::ServeError,
};

fn get_device_id_from_headers(req: &HttpRequest) -> Result<&str, ServeError> {
    req.headers()
        .get(ID_HEADER)
        .ok_or(ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            "Missing ID header. This typically contains the device's MAC address.".to_string(),
        ))?
        .to_str()
        .map_err(|e| {
            ServeError::UserFacing(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse ID header as a string: {}", e),
            )
        })
}

fn get_api_key_from_headers(req: &HttpRequest) -> Result<&str, ServeError> {
    req.headers()
        .get(ACCESS_TOKEN_HEADER)
        .ok_or(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            format!("Missing {ACCESS_TOKEN_HEADER} header."),
        ))?
        .to_str()
        .map_err(|e| {
            ServeError::UserFacing(
                StatusCode::BAD_REQUEST,
                format!(
                    "Failed to parse {ACCESS_TOKEN_HEADER} header as a string: {}",
                    e
                ),
            )
        })
}

pub(super) fn try_get_api_key_from_headers(
    headers: &HeaderMap,
) -> Result<Option<&str>, ServeError> {
    headers
        .get(ACCESS_TOKEN_HEADER)
        .map(|v| {
            v.to_str().map_err(|e| {
                ServeError::UserFacing(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Failed to parse {ACCESS_TOKEN_HEADER} header as a string: {}",
                        e
                    ),
                )
            })
        })
        .transpose()
}

fn authenticate_device<'d>(
    id: &str,
    req: &HttpRequest,
    device: &'d Device,
    show_api_keys: ShowApiKeys,
) -> Result<&'d TrmnlDevice, ServeError> {
    let Some(trmnl_device) = device.trmnl.as_ref() else {
        return Err(ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            "Device does not have a terminal configuration.".to_string(),
        ));
    };

    let api_key = get_api_key_from_headers(req)?;

    let hashed_api_key = hash_string(api_key);
    let hashed_id = hash_string(id);
    if hashed_id != trmnl_device.hashed_id || hashed_api_key != trmnl_device.hashed_api_key {
        debug!("Device authentication failed.");
        debug!(
            "Expected hashed ID: {}, received: {}",
            trmnl_device.hashed_id, hashed_id
        );
        debug!(
            "Expected hashed key: {}, received: {}",
            trmnl_device.hashed_api_key, hashed_api_key
        );

        if matches!(show_api_keys, ShowApiKeys::Always) {
            write_api_key_message(api_key);
        }

        return Err(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials.".to_string(),
        ));
    }

    Ok(trmnl_device)
}

fn get_optional_header<'a>(req: &'a HttpRequest, header: &str) -> Option<&'a str> {
    req.headers()
        .get(header)
        .and_then(|header| header.to_str().ok())
}

fn print_new_device_message(
    hashed_id: &str,
    hashed_api_key: &str,
    unhashed_data: Option<UnhashedData>,
    existing_device_name: Option<DeviceName>,
) {
    info!("To allow this device, run the following command from your Slipway serve root:");
    info!("");
    info!("  slipway serve . add-trmnl-device \\");

    if let Some(device_name) = existing_device_name {
        info!("    --name \"{device_name}\" \\");
    } else {
        info!("    --name \"<NAME>\" \\");
    }

    info!("    --hashed-id \"{hashed_id}\" \\");
    info!("    --hashed-api-key \"{hashed_api_key}\" \\");
    info!("    --playlist <PLAYLIST>");
    info!("");
    info!("Then re-deploy the server if necessary.");

    if let Some(unhashed_data) = unhashed_data {
        info!("The ID key sent by the device was: {}", unhashed_data.id);
        info!(
            "The API key sent by the device was: {}",
            unhashed_data.api_key
        );
        info!(
            "The ID and API key are not stored by the server. If you need a record of them, store them securely now."
        );
    }
    info!("See the Slipway documentation for more information.");
}

struct UnhashedData<'a> {
    id: &'a str,
    api_key: &'a str,
}
