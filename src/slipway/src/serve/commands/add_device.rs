use std::path::PathBuf;

use tracing::warn;

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{create_repository, load_serve_config, repository::Device},
};

pub async fn add_device(
    serve_path: PathBuf,
    name: DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    warn!("Adding device \"{name}\".");
    warn!("Don't forget to re-deploy if necessary.");

    let device = Device {
        trmnl: None,
        playlist,
        context: None,
    };

    repository.set_device(&name, &device).await?;

    Ok(())
}
