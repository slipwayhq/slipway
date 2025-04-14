use std::path::PathBuf;

use tracing::warn;

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{create_repository, load_serve_config, repository::Device, write_redeploy_warning},
};

pub async fn add_device(
    serve_path: PathBuf,
    name: DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    warn!("Adding device \"{name}\".");
    write_redeploy_warning();

    let device = Device {
        trmnl: None,
        playlist,
        context: None,
    };

    repository.set_device(&name, &device).await?;

    Ok(())
}
