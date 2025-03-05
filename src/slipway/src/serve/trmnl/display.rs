use actix_web::{get, web, HttpRequest, Responder};
use tracing::{debug, instrument, warn};

use crate::serve::{
    repository::Device,
    trmnl::{authenticate_device, get_device_id},
    RigResponse, RigResultImageFormat, RigResultPresentation, ServeError, ServeState,
};

use super::get_optional_header;

#[get("/display")]
#[instrument(name = "trmnl_display")]
pub(crate) async fn trmnl_display(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id(&req)?;
    let device = data.repository.get_device_by_id(id).await?;
    authenticate_device(id, &req, &device)?;

    print_optional_headers(&req, &device);

    let device_name = device.name;

    if device.reset_firmware {
        warn!(
            "Device \"{}\" JSON configuration is set to trigger a firmware reset.",
            device_name
        );
    }

    let device_response = super::super::devices::get_device::get_device_response(
        &device_name,
        RigResultImageFormat::Bmp1Bit,
        RigResultPresentation::Url,
        data.into_inner(),
        req,
    )
    .await?;

    let RigResponse::Url(url_response) = device_response.rig_response else {
        panic!("Expected URL response from device.");
    };

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "image_url": url_response.url,
        "filename": format!("{}.bmp", device_name),
        "update_firmware": false,
        "firmware_url": serde_json::Value::Null,
        "refresh_rate": device_response.refresh_rate_seconds,
        "reset_firmware": device.reset_firmware
    })))
}

fn print_optional_headers(req: &HttpRequest, device: &Device) {
    if let Some(battery_voltage) = get_optional_header(req, "Battery-Voltage") {
        debug!(
            "Battery voltage for \"{}\": {}",
            device.name, battery_voltage
        );
    }

    if let Some(rssi) = get_optional_header(req, "RSSI") {
        debug!("RSSI for \"{}\": {}", device.name, rssi);
    }

    if let Some(fw_version) = get_optional_header(req, "FW-Version") {
        debug!("Firmware version for \"{}\": {}", device.name, fw_version);
    }
}
