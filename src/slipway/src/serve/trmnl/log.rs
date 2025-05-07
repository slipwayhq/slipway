use actix_web::{HttpRequest, Responder, post, web};
use slipway_host::hash_string;
use tracing::{info, instrument};

use crate::serve::{
    ServeState, ShowApiKeys,
    responses::ServeError,
    trmnl::{authenticate_device, get_api_key_from_headers, get_device_id_from_headers},
    write_api_key_message,
};

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

    let id = get_device_id_from_headers(&req);
    let device_name = match id {
        Ok(id) => {
            let hashed_id = hash_string(id);
            let (device_name, device) = data.repository.get_device_by_hashed_id(&hashed_id).await?;
            authenticate_device(id, &req, &device, data.config.show_api_keys)?;
            device_name
        }
        Err(_e) => {
            // Older TRMNL firmware doesn't supply the device ID in the log headers.
            let api_key = get_api_key_from_headers(&req)?;

            if matches!(data.config.show_api_keys, ShowApiKeys::Always) {
                write_api_key_message(api_key);
            }

            let (device_name, _device) = data.repository.get_device_by_api_key(api_key).await?;
            device_name
        }
    };

    info!("Log from device \"{device_name}\": {log_text}",);

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "message": "Log received.",
    })))
}
