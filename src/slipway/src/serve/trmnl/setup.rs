use actix_web::{get, http::StatusCode, web, HttpRequest, Responder};
use tracing::{instrument, warn};

use crate::serve::{
    create_api_key, create_friendly_id, hash_string, trmnl::get_device_id, ServeError, ServeState,
};

#[get("/setup")]
#[instrument(name = "trmnl_setup")]
pub(crate) async fn trmnl_setup(
    data: web::Data<ServeState>,
    req: HttpRequest,
) -> Result<impl Responder, ServeError> {
    let id = get_device_id(&req)?;

    warn!("A request to setup a device with ID \"{id}\" was received.");

    let maybe_device = data.repository.try_get_device_by_id(id).await?;

    if let Some(_device) = maybe_device {
        warn!("This device already exists, so the request was ignored.");

        // Already set up, so act as though the device doesn't exist.
        return Err(ServeError::UserFacingJson(
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "status": 404,
                "api_key": serde_json::Value::Null,
                "friendly_id": serde_json::Value::Null,
                "image_url": serde_json::Value::Null,
                "filename": serde_json::Value::Null,
            }),
        ));
    }

    // Our self-host web server is immutable, so we can't store an API key anywhere. Instead, we assign a random
    // API key and leave it up to the user to either add the device to the server and re-deploy, or just ignore it.
    let api_key = create_api_key();
    let hashed_api_key = hash_string(&api_key);
    let friendly_id = create_friendly_id();

    warn!("Random credentials have been generated for the device.");
    warn!(
        "If you wish to allow this device, run the following command from your Slipway serve root:"
    );
    warn!("");
    warn!("  slipway serve . add-trmnl-device \\");
    warn!("    --name \"<NAME>\" \\");
    warn!("    --id \"{id}\" \\");
    warn!("    --friendly-id \"{friendly_id}\" \\");
    warn!("    --hashed-api-key \"{hashed_api_key}\" \\");
    warn!("    --playlist <PLAYLIST?>");
    warn!("");
    warn!("Then re-deploy the server if necessary.");
    warn!("The API key sent to the device was \"{api_key}\".");
    warn!(
        "The API key is not stored by the server. If you need a record of it, store it securely."
    );
    warn!("If you do not wish to allow this device then you can safely ignore this message.");
    warn!("See the Slipway documentation for more information.");

    Ok(web::Json(serde_json::json!({
        "status": 200,
        "api_key": api_key,
        "friendly_id": friendly_id,
        "image_url": serde_json::Value::Null,
        "filename": serde_json::Value::Null,
        "message": "Device {device.friendly_id} added.",
    })))
}
