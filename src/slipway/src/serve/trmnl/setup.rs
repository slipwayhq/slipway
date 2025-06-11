use actix_web::{HttpRequest, Responder, get, web};
use slipway_host::hash_string;
use tracing::{info, instrument, warn};

use crate::serve::{
    ServeState, ShowApiKeys, create_api_key,
    responses::ServeError,
    trmnl::{get_device_id_from_headers, print_new_device_message},
    truncate_hashed_api_key,
};

#[get("/setup")]
#[instrument(name = "trmnl_setup", skip_all)]
pub(crate) async fn trmnl_setup(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id_from_headers(&req)?;

    warn!("A request to setup a device with ID \"{id}\" was received.");

    info!("Random credentials have been generated for the device.");
    info!("If you do not wish to allow this device then you can safely ignore the request.");

    // Our self-host web server is immutable, so we can't store an API key anywhere. Instead, we assign a random
    // API key and leave it up to the administrator to either add the device to the server and re-deploy, or just ignore it.
    let api_key = create_api_key();
    let hashed_api_key = hash_string(&api_key);
    let friendly_id = truncate_hashed_api_key(&hashed_api_key);

    let unhashed_data = match data.config.show_api_keys {
        ShowApiKeys::Always | ShowApiKeys::New => Some(super::UnhashedData { api_key: &api_key }),
        ShowApiKeys::Never => None,
    };

    print_new_device_message(&hashed_api_key, unhashed_data);

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "api_key": api_key,
        "friendly_id": friendly_id,
        "message": "New credentials generated.",
    })))
}
