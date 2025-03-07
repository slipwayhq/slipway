use actix_web::{post, web, HttpRequest, Responder};
use tracing::{info, instrument};

use crate::serve::{
    trmnl::{authenticate_device, get_device_id_from_headers},
    ServeError, ServeState,
};

// TODO: Logging seems to be broken. I don't think this method is getting called.

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

    info!("Log from device: {log_text}",);

    let id = get_device_id_from_headers(&req)?;
    let (device_name, device) = data.repository.get_device_by_id(id).await?;
    authenticate_device(id, &req, &device)?;

    info!("Log from device \"{device_name}\": {log_text}",);

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "message": "Log received.",
    })))
}
