use std::path::PathBuf;

use tracing::warn;

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{
        create_repository, load_serve_config,
        repository::{Device, TrmnlDevice},
        write_redeploy_warning,
    },
};

pub async fn add_trmnl_device(
    serve_path: PathBuf,
    hashed_id: String,
    hashed_api_key: String,
    name: DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let existing_device_by_id = repository.try_get_device_by_hashed_id(&hashed_id).await?;
    let existing_device = if let Some((existing_name, existing_device)) = existing_device_by_id {
        if existing_name != name {
            anyhow::bail!(
                "Device with hashed ID {hashed_id} already exists with name {existing_name}.",
            );
        }
        Some(existing_device)
    } else {
        repository.try_get_device(&name).await?
    };

    if let Some(mut existing_device) = existing_device {
        warn!("Updating existing device \"{name}\".");

        existing_device.trmnl = Some(TrmnlDevice {
            hashed_id,
            hashed_api_key,
        });

        if let Some(playlist) = playlist {
            existing_device.playlist = Some(playlist);
        }

        repository.set_device(&name, &existing_device).await?;
    } else {
        warn!("Adding device \"{name}\".");

        let device = Device {
            trmnl: Some(TrmnlDevice {
                hashed_id,
                hashed_api_key,
            }),
            playlist,
            context: None,
        };

        repository.set_device(&name, &device).await?;
    }

    write_redeploy_warning();

    Ok(())
}
