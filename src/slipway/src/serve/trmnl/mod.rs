mod display;
mod log;
mod setup;

use actix_web::{http::StatusCode, HttpRequest};
pub(super) use display::trmnl_display;
pub(super) use log::trmnl_log;
pub(super) use setup::trmnl_setup;
use tracing::debug;

use crate::serve::hash_string;

use super::{repository::Device, ServeError};

fn get_device_id(req: &HttpRequest) -> Result<&str, ServeError> {
    req.headers()
        .get("ID")
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

fn authenticate_device(id: &str, req: &HttpRequest, device: &Device) -> Result<(), ServeError> {
    const ACCESS_TOKEN_HEADER: &str = "Access-Token";

    let api_key = req
        .headers()
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
        })?;

    let hashed_api_key = hash_string(api_key);
    if id != device.id || hashed_api_key != device.hashed_api_key {
        debug!("Device authentication failed.");
        debug!("Expected ID: {}, received: {}", device.id, id);
        debug!(
            "Expected hashed key: {}, received: {}",
            device.hashed_api_key, hashed_api_key
        );

        return Err(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials.".to_string(),
        ));
    }

    Ok(())
}

fn get_optional_header<'a>(req: &'a HttpRequest, header: &str) -> Option<&'a str> {
    req.headers()
        .get(header)
        .and_then(|header| header.to_str().ok())
}
