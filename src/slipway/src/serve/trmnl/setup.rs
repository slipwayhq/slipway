use actix_web::{get, http::StatusCode, web, HttpRequest, Responder};

use crate::serve::{ServeError, ServeState};

#[get("/setup")]
pub(crate) async fn trmnl_setup(
    _data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = req
        .headers()
        .get("ID")
        .ok_or(ServeError::UserFacing(
            StatusCode::BAD_REQUEST,
            "Missing ID header. This typically contains the device's MAC address.".to_string(),
        ))?
        .to_str()
        .map_err(|e| {
            ServeError::UserFacing(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse ID header: {}", e),
            )
        })?;

    todo!();
}
