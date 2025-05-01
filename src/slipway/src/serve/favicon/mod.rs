use actix_web::{HttpResponse, Responder, get};

const FAVICON: &[u8] = include_bytes!("favicon.ico");

#[get("/favicon.ico")]
pub(super) async fn get_favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/x-icon")
        .body(FAVICON)
}
