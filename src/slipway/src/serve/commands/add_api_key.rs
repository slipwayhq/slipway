use std::path::{Path, PathBuf};

use slipway_host::hash_string;
use termion::color;
use tracing::{info, warn};

use crate::{
    primitives::{DeviceName, PlaylistName},
    serve::{
        Device, RegisteredApiKey, SlipwayServeConfig, create_api_key, create_repository,
        load_serve_config, save_serve_config, write_redeploy_warning,
    },
};

pub async fn add_api_key(
    serve_path: PathBuf,
    api_key: Option<String>,
    hashed_api_key: Option<String>,
    description: Option<String>,
    device: Option<DeviceName>,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let mut config = load_serve_config(&serve_path).await?;

    if api_key.is_some() {
        warn!(
            "You have provided your own API key. Please ensure it is sufficiently complex and random."
        );
    }

    let (api_key, hashed_key) = match (api_key, hashed_api_key) {
        (Some(api_key), None) => {
            let hashed_key = hash_string(&api_key);
            (Some(api_key), hashed_key)
        }
        (None, Some(hashed)) => (None, hashed),
        (None, None) => {
            let api_key = create_api_key();
            let hashed_key = hash_string(&api_key);
            (Some(api_key), hashed_key)
        }
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!(
                "You cannot provide both an API key and a hashed API key."
            ));
        }
    };

    if let Some(device_name) = device.as_ref() {
        add_associated_device(&config, &serve_path, device_name, playlist).await?;
    }

    if let Some(existing_key) = config
        .api_keys
        .iter_mut()
        .find(|key| key.hashed_key == hashed_key)
    {
        // If an item already exists with the same hashed key, update its properties.
        info!(
            "Updating existing API key with hashed value {}.",
            hashed_key
        );

        if let Some(description) = &description {
            existing_key.description = Some(description.clone());
        }

        if let Some(device) = device {
            existing_key.device = Some(device.clone());
        }
    } else {
        // Otherwise, add a new API key.
        if let Some(api_key) = api_key {
            info!(
                "Adding API key {}{}{} with hashed value {} to Slipway Serve config.",
                color::Fg(color::Green),
                api_key,
                color::Fg(color::Reset),
                hashed_key
            );
        } else {
            info!(
                "Adding API key with hashed value {} to Slipway Serve config.",
                hashed_key
            );
        }

        config.api_keys.push(RegisteredApiKey {
            hashed_key,
            device,
            description,
        });
    }

    save_serve_config(&serve_path, &config).await?;

    warn!(
        "The unhashed API key is not stored by the server. If you need a record of it, store it securely."
    );

    write_redeploy_warning();

    Ok(())
}

pub async fn add_associated_device(
    config: &SlipwayServeConfig,
    serve_path: &Path,
    name: &DeviceName,
    playlist: Option<PlaylistName>,
) -> anyhow::Result<()> {
    let repository = create_repository(serve_path, &config.repository);

    let existing_device = repository.try_get_device(name).await?;
    if let Some(mut existing_device) = existing_device {
        warn!("Updating existing device \"{name}\".");

        if let Some(playlist) = playlist {
            existing_device.playlist = Some(playlist);
        }

        repository.set_device(name, &existing_device).await?;
    } else {
        warn!("Adding device \"{name}\".");

        let device = Device {
            playlist,
            context: None,
            result_spec: Default::default(),
        };

        repository.set_device(name, &device).await?;
    }

    Ok(())
}
