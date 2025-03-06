use actix_web::{get, web, HttpRequest, Responder};
use tracing::instrument;

use crate::serve::{
    trmnl::{authenticate_device, get_device_id},
    ServeError, ServeState,
};

#[get("/log")]
#[instrument(name = "trmnl_log")]
pub(crate) async fn trmnl_log(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id(&req)?;
    let (_device_name, device) = data.repository.get_device_by_id(id).await?;
    authenticate_device(id, &req, &device)?;

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "message": "Log received.",
    })))
}
