use actix_web::{get, http::StatusCode, web, HttpRequest, Responder};
use tracing::{info, instrument};

use crate::serve::{
    trmnl::{authenticate_device, get_device_id_from_headers},
    ServeError, ServeState,
};

#[get("/log")]
#[instrument(name = "trmnl_log", skip_all)]
pub(crate) async fn trmnl_log(
    data: web::Data<ServeState>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id_from_headers(&req)?;
    let (device_name, device) = data.repository.get_device_by_id(id).await?;
    authenticate_device(id, &req, &device)?;

    match String::from_utf8(body.to_vec()) {
        Ok(text) => {
            info!("Log from device \"{device_name}\": {text}",);
            Ok(web::Json(serde_json::json!({
                "status": 200,
                "message": "Log received.",
            })))
        }
        Err(e) => Err(ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            format!("Unable to parse log as UTF-8.\n{e}"),
        )),
    }
}
