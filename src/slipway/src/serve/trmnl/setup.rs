use actix_web::{get, web, HttpRequest, Responder};
use tracing::{info, instrument, warn};

use crate::serve::{
    create_api_key, create_friendly_id, hash_string,
    trmnl::{get_device_id_from_headers, print_new_device_message},
    Device, ServeError, ServeState,
};

#[get("/setup")]
#[instrument(name = "trmnl_setup", skip_all)]
pub(crate) async fn trmnl_setup(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id_from_headers(&req)?;

    warn!("A request to setup a device with ID \"{id}\" was received.");

    let maybe_device = data.repository.try_get_device_by_id(id).await?;

    let existing_device_name = if let Some((
        device_name,
        Device {
            trmnl: Some(_trmnl),
            ..
        },
    )) = maybe_device
    {
        // We return generated credentials for devices even if they already have a TRMNL configuration.
        // This makes it easier to update the credentials of devices which have been reset.
        warn!("The device \"{device_name}\" already contains a TRMNL configuration for this ID.");
        warn!("New random credentials will be returned.");
        warn!("If this is not a genuine request then you can safely ignore it.");
        warn!(
            "Otherwise you can follow the instructions below to update the device configuration."
        );
        Some(device_name)
    } else {
        info!("Random credentials have been generated for the device.");
        info!("If you do not wish to allow this device then you can safely ignore the request.");
        None
    };

    // Our self-host web server is immutable, so we can't store an API key anywhere. Instead, we assign a random
    // API key and leave it up to the user to either add the device to the server and re-deploy, or just ignore it.
    let api_key = create_api_key();
    let hashed_api_key = hash_string(&api_key);
    let friendly_id = create_friendly_id(&hashed_api_key);

    print_new_device_message(id, &api_key, &hashed_api_key, existing_device_name);

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "api_key": api_key,
        "friendly_id": friendly_id,
        "image_url": serde_json::Value::Null,
        "filename": serde_json::Value::Null,
        "message": "New credentials generated.",
    })))
}
