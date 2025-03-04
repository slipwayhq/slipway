use std::path::PathBuf;

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

    warn!("Adding device with the following properties:");
    warn!(" ID: {id}");
    warn!(" Friendly ID: {friendly_id}");
    warn!(" Name: {name}");
    warn!("");
    warn!("Don't forget to re-deploy if necessary.");

    let device = Device {
        id,
        friendly_id,
        hashed_api_key,
        name: name.clone(),
        playlist,
        context: serde_json::json!({}),
        reset_firmware: false,
    };

    repository.set_device(&name, &device).await?;

    Ok(())
}
