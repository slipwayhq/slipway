use std::path::PathBuf;

use chrono::{DateTime, Duration, Local};
use tracing::warn;

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{create_repository, load_serve_config, repository::Device},
};

pub async fn add_device(
    serve_path: PathBuf,
    id: String,
    friendly_id: String,
    hashed_api_key: String,
    name: DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let local_now: DateTime<Local> = Local::now();
    let allow_setup_until = local_now + Duration::days(1);

    warn!("Adding device with the following properties:");
    warn!(" ID: {id}");
    warn!(" Friendly ID: {friendly_id}");
    warn!(" Name: {name}");
    warn!("");
    warn!(
        "Allowing this device to be setup until: {}",
        allow_setup_until.to_rfc2822()
    );
    warn!("You may adjust this by editing the device JSON file");
    warn!("");
    warn!("Don't forget to re-deploy if necessary.");

    let device = Device {
        id: id.clone(),
        friendly_id,
        hashed_api_key,
        name,
        playlist,
        context: serde_json::json!({}),
        reset_firmware: false,
        allow_setup_until: Some(allow_setup_until.into()),
    };

    repository.set_device(&id, &device).await?;

    Ok(())
}
