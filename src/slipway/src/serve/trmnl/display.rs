use actix_web::{HttpRequest, Responder, get, web};
use slipway_host::hash_string;
use tracing::{Instrument, debug, info_span, instrument, warn};

use crate::{
    primitives::DeviceName,
    serve::{
        RegisteredApiKey, ServeState, ShowApiKeys, SlipwayServeConfig, get_api_key_from_state,
        repository::{RigResultFormat, RigResultImageFormat, RigResultSpec},
        responses::{FormatQuery, RigResponse, ServeError},
        truncate_hashed_api_key,
    },
};

use super::get_optional_header;

#[get("/display")]
#[instrument(name = "trmnl_display", skip_all)]
pub(crate) async fn trmnl_display(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let supplied_api_key = get_api_key_from_state(&req)?;

    let Some(resolved) = supplied_api_key.resolved else {
        return Err(print_unknown_device_message(
            &supplied_api_key.api_key,
            &data.config,
        ));
    };

    debug!(
        "A display request was received from a device with hashed API key: {}",
        truncate_hashed_api_key(&resolved.hashed_key)
    );

    let Some(device_name) = resolved.device else {
        return Err(print_no_linked_device_message(&resolved));
    };

    print_optional_headers(&req, &device_name);

    let device_response = super::super::devices::get_device::get_device_response(
        &device_name,
        FormatQuery::none(),
        Some(RigResultSpec {
            format: RigResultFormat::Url,
            image_format: RigResultImageFormat::Bmp1Bit,
            rotate: 0,
        }),
        data.into_inner(),
        req,
    )
    .instrument(info_span!("device", ""=%device_name))
    .await?;

    let RigResponse::Url(url_response) = device_response.rig_response else {
        panic!("Expected URL response from device.");
    };

    Ok(web::Json(serde_json::json!({
        "status": 0,
        "image_url": url_response.url,
        "image_url_timeout": 10, // Make the timeout a bit more generous. We should make this configurable.
        "filename": chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string(),
        "refresh_rate": device_response.refresh_rate_seconds,
    })))
}

fn print_unknown_device_message(api_key: &str, config: &SlipwayServeConfig) -> ServeError {
    let hashed_api_key = hash_string(api_key);

    let unhashed_data = match config.show_api_keys {
        ShowApiKeys::Always | ShowApiKeys::New => Some(super::UnhashedData { api_key }),
        ShowApiKeys::Never => None,
    };

    warn!("An device called the TRMNL display API with an unrecognized API key.");
    super::print_new_device_message(&hashed_api_key, unhashed_data);

    ServeError::UserFacing(
        actix_web::http::StatusCode::UNAUTHORIZED,
        "The supplied API key was not recognized.".to_string(),
    )
}

fn print_no_linked_device_message(resolved_api_key: &RegisteredApiKey) -> ServeError {
    warn!("TRMNL display API called with an API key which is not linked to any device.",);
    super::print_update_key_message(&resolved_api_key.hashed_key);

    ServeError::UserFacing(
        actix_web::http::StatusCode::FORBIDDEN,
        "The supplied API key was not associated with any device.".to_string(),
    )
}

fn print_optional_headers(req: &HttpRequest, device_name: &DeviceName) {
    if let Some(battery_voltage) = get_optional_header(req, "Battery-Voltage") {
        debug!(
            "Battery voltage for \"{}\": {}",
            device_name, battery_voltage
        );
    }

    if let Some(rssi) = get_optional_header(req, "RSSI") {
        debug!("RSSI for \"{}\": {}", device_name, rssi);
    }

    if let Some(fw_version) = get_optional_header(req, "FW-Version") {
        debug!("Firmware version for \"{}\": {}", device_name, fw_version);
    }
}
