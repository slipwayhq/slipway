use actix_web::{HttpRequest, Responder, get, web};
use slipway_host::hash_string;
use tracing::{Instrument, debug, info_span, instrument, warn};

use crate::{
    primitives::DeviceName,
    serve::{
        ServeState, ShowApiKeys, SlipwayServeConfig,
        responses::{FormatQuery, RigResponse, ServeError},
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

    let hashed_id = hash_string(id);

    let maybe_device = data
        .repository
        .try_get_device_by_hashed_id(&hashed_id)
        .await?;
    let (device_name, device) = if let Some((device_name, device)) = maybe_device {
        (device_name, device)
    } else {
        return Err(print_unknown_device_message(&req, id, &data.config)?);
    };

    let _trmnl_device = authenticate_device(id, &req, &device, data.config.show_api_keys)?;

    print_optional_headers(&req, &device_name);

    let device_response = super::super::devices::get_device::get_device_response(
        &device_name,
        FormatQuery::none(),
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

fn print_unknown_device_message(
    req: &HttpRequest,
    id: &str,
    config: &SlipwayServeConfig,
) -> Result<ServeError, ServeError> {
    let api_key = get_api_key_from_headers(req)?;
    let hashed_id = hash_string(id);
    let hashed_api_key = hash_string(api_key);

    let unhashed_data = match config.show_api_keys {
        ShowApiKeys::Always | ShowApiKeys::New => Some(super::UnhashedData { api_key, id }),
        ShowApiKeys::Never => None,
    };

    warn!("An unknown device with ID \"{id}\" called the TRMNL display API.");
    super::print_new_device_message(&hashed_id, &hashed_api_key, unhashed_data, None);

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
