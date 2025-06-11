use actix_web::{HttpRequest, Responder, post, web};
use tracing::{info, instrument};

use crate::serve::{
    ServeState, ShowApiKeys, get_resolved_api_key_from_state, responses::ServeError,
    truncate_hashed_api_key, write_api_key_message,
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

    let supplied_api_key = get_resolved_api_key_from_state(&req)?;

    if matches!(data.config.show_api_keys, ShowApiKeys::Always) {
        write_api_key_message(&supplied_api_key.api_key);
    }

    if let Some(device_name) = supplied_api_key.resolved.device {
        info!("Log from device \"{device_name}\": {log_text}",);
    } else {
        info!(
            "Log from hashed API key {} which is not associated with any device: {log_text}",
            truncate_hashed_api_key(&supplied_api_key.api_key)
        );
    }

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "message": "Log received.",
    })))
}
