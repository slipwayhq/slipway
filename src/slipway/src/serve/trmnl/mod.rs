mod display;
mod log;
mod setup;

use actix_web::{HttpRequest, http::StatusCode};
pub(super) use display::trmnl_display;
pub(super) use log::trmnl_log;
pub(super) use setup::trmnl_setup;
use termion::color;
use tracing::info;

use super::{ID_HEADER, responses::ServeError};

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

fn get_optional_header<'a>(req: &'a HttpRequest, header: &str) -> Option<&'a str> {
    req.headers()
        .get(header)
        .and_then(|header| header.to_str().ok())
}

fn print_new_device_message(hashed_api_key: &str, unhashed_data: Option<UnhashedData>) {
    info!("To allow this device, run the following command from your Slipway serve root:");
    info!("");
    info!("  slipway serve . add-api-key \\");
    info!("    --hashed-key \"{hashed_api_key}\" \\");
    info!("    --device <DEVICE_NAME>");
    info!("    --playlist <PLAYLIST_NAME>");
    info!("");
    info!("Then re-deploy the server if necessary.");

    if let Some(unhashed_data) = unhashed_data {
        info!(
            "The unhashed API key is: {}{}{}",
            color::Fg(color::Green),
            unhashed_data.api_key,
            color::Fg(color::Reset),
        );
        info!(
            "The API key is not stored by the server. If you need a record of it, store it securely now."
        );
    }
    info!("See the Slipway documentation for more information.");
}

fn print_update_key_message(hashed_api_key: &str) {
    info!("To link to a device, run the following command from your Slipway serve root:");
    info!("");
    info!("  slipway serve . add-api-key \\");
    info!("    --hashed-key \"{hashed_api_key}\" \\");
    info!("    --device <DEVICE_NAME>");
    info!("    --playlist <PLAYLIST_NAME>");
    info!("");
    info!("Then re-deploy the server if necessary.");
    info!("See the Slipway documentation for more information.");
}

struct UnhashedData<'a> {
    api_key: &'a str,
}
