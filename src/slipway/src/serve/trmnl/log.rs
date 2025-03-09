use actix_web::{post, web, HttpRequest, Responder};
use tracing::{info, instrument};

use crate::serve::{trmnl::get_api_key_from_headers, ServeError, ServeState};

#[post("/log")]
#[instrument(name = "trmnl_log", skip_all)]
pub(crate) async fn trmnl_log(
    data: web::Data<ServeState>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<impl Responder, ServeError> {
    let log_text = match String::from_utf8(body.to_vec()) {
        Ok(text) => text,
        Err(e) => format!("Failed to parse log text as UTF-8\n{e}"),
    };

    // Firmware doesn't send ID with log messages, so we can't look up the device by ID.
    // let id = get_device_id_from_headers(&req)?;
    // let (device_name, device) = data.repository.get_device_by_id(id).await?;
    // authenticate_device(id, &req, &device)?;

    let api_key = get_api_key_from_headers(&req)?;
    let (device_name, _device) = data.repository.get_device_by_api_key(api_key).await?;

    info!("Log from device \"{device_name}\": {log_text}",);

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "message": "Log received.",
    })))
}
