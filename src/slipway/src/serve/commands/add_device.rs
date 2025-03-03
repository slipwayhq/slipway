use std::path::PathBuf;

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{create_repository, hash_string, load_serve_config, repository::Device},
};

use super::{create_api_key, create_friendly_id};

pub async fn add_device(
    serve_path: PathBuf,
    id: String,
    name: DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let friendly_id = create_friendly_id();
    let api_key = create_api_key();
    let hashed_api_key = hash_string(&api_key);

    let device = Device {
        friendly_id,
        hashed_api_key,
        name,
        playlist,
        context: serde_json::json!({}),
    };

    repository.set_device(&id, &device).await?;

    Ok(())
}
