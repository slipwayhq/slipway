use actix_web::{get, web, HttpRequest, Responder};
use tracing::{info, instrument, warn};

use crate::serve::{
    create_api_key, create_friendly_id, hash_string, trmnl::get_device_id_from_headers, Device,
    ServeError, ServeState,
};

#[get("/setup")]
#[instrument(name = "trmnl_setup")]
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
        // We return credentials to the device even if it already has a TRMNL configuration.
        // This makes it impossible for a third party to tell what device MAC addresses exist
        // through brute force using this API.
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
    let friendly_id = create_friendly_id();

    info!("To allow this device run the following command from your Slipway serve root:");
    info!("");
    info!("  slipway serve . add-trmnl-device \\");

    if let Some(device_name) = existing_device_name {
        info!("    --name \"{device_name}\" \\");
    } else {
        info!("    --name \"<NAME>\" \\");
    }

    info!("    --id \"{id}\" \\");
    info!("    --friendly-id \"{friendly_id}\" \\");
    info!("    --hashed-api-key \"{hashed_api_key}\" \\");
    info!("    --playlist <?PLAYLIST?>");
    info!("");
    info!("Then re-deploy the server if necessary.");
    info!("The API key sent to the device was: {api_key}");
    info!(
        "The API key is not stored by the server. If you need a record of it, store it securely."
    );
    info!("See the Slipway documentation for more information.");

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "api_key": api_key,
        "friendly_id": friendly_id,
        "image_url": serde_json::Value::Null,
        "filename": serde_json::Value::Null,
        "message": "New credentials generated.",
    })))
}
