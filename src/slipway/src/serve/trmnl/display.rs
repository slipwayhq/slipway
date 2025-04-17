use actix_web::{HttpRequest, Responder, get, web};
use slipway_host::hash_string;
use tracing::{Instrument, debug, info_span, instrument, warn};

use crate::{
    primitives::DeviceName,
    serve::{
        ServeState, ShowApiKeys, SlipwayServeConfig,
        responses::{RigResponse, RigResultFormat, RigResultImageFormat, ServeError},
        trmnl::{authenticate_device, get_api_key_from_headers, get_device_id_from_headers},
    },
};

use super::get_optional_header;

#[get("/display")]
#[instrument(name = "trmnl_display", skip_all)]
pub(crate) async fn trmnl_display(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id_from_headers(&req)?;
    debug!(
        "A display request was received from a device with ID \"{}\".",
        id
    );

    let maybe_device = data.repository.try_get_device_by_id(id).await?;
    let (device_name, device) = if let Some((device_name, device)) = maybe_device {
        (device_name, device)
    } else {
        return Err(print_unknown_device_message(&req, id, &data.config)?);
    };

    let trmnl_device = authenticate_device(id, &req, &device, data.config.show_api_keys)?;

    // We check this before authenticating, so that if the device set itself up
    // and we didn't get the hashed API key we can still reset the firmware.
    if trmnl_device.reset_firmware {
        warn!(
            "Device \"{}\" JSON configuration is set to trigger a firmware reset. This will be done now.",
            device_name
        );

        return Ok(web::Json(serde_json::json!({
            "status": 0,
            "image_url": serde_json::Value::Null,
            "filename": serde_json::Value::Null,
            "update_firmware": false,
            "firmware_url": serde_json::Value::Null,
            "refresh_rate": serde_json::Value::Null,
            "reset_firmware": true
        })));
    }

    print_optional_headers(&req, &device_name);

    let device_response = super::super::devices::get_device::get_device_response(
        &device_name,
        RigResultFormat::Url,
        RigResultImageFormat::Bmp1Bit,
        data.into_inner(),
        req,
    )
    .instrument(info_span!("device", %device_name))
    .await?;

    let RigResponse::Url(url_response) = device_response.rig_response else {
        panic!("Expected URL response from device.");
    };

    Ok(web::Json(serde_json::json!({
        "status": 0,
        "image_url": url_response.url,
        "image_url_timeout": 10, // Make the timeout a bit more generous. We should make this configurable.
        "filename": chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string(),
        "update_firmware": false,
        "firmware_url": serde_json::Value::Null,
        "refresh_rate": device_response.refresh_rate_seconds,
        "reset_firmware": false,
    })))
}

fn print_unknown_device_message(
    req: &HttpRequest,
    id: &str,
    config: &SlipwayServeConfig,
) -> Result<ServeError, ServeError> {
    let api_key = get_api_key_from_headers(req)?;
    let hashed_api_key = hash_string(api_key);

    let display_api_key = match config.show_api_keys {
        ShowApiKeys::Always => Some(api_key),
        ShowApiKeys::New => Some(api_key),
        ShowApiKeys::Never => None,
    };

    warn!("An unknown device with ID \"{id}\" called the TRMNL display API.");
    super::print_new_device_message(id, display_api_key, &hashed_api_key, None);

    Ok(ServeError::UserFacing(
        actix_web::http::StatusCode::NOT_FOUND,
        format!("No device with ID \"{}\".", id),
    ))
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
