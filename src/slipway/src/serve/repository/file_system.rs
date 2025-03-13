use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use actix_web::http::StatusCode;
use anyhow::Context;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tracing::warn;

use crate::{
    primitives::{DeviceName, PlaylistName, RigName},
    serve::ServeError,
};

use super::{Device, Playlist, ServeRepository};

#[derive(Clone, Debug)]
pub(crate) struct FileSystemRepository {
    root_path: PathBuf,
}

impl FileSystemRepository {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }
}

#[async_trait(?Send)]
impl ServeRepository for FileSystemRepository {
    async fn get_rig(&self, name: &RigName) -> Result<slipway_engine::Rig, ServeError> {
        let path = get_rig_path(&self.root_path, name);
        load_from_file(&path, "Rig").await
    }

    async fn set_rig(&self, name: &RigName, value: &slipway_engine::Rig) -> Result<(), ServeError> {
        let path = get_rig_path(&self.root_path, name);
        write_to_file(&path, "Rig", value).await
    }

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError> {
        let path = get_playlist_path(&self.root_path, name);
        load_from_file(&path, "Playlist").await
    }

    async fn set_playlist(&self, name: &PlaylistName, value: &Playlist) -> Result<(), ServeError> {
        let path = get_playlist_path(&self.root_path, name);
        write_to_file(&path, "Playlist", value).await
    }
    async fn get_device(&self, name: &DeviceName) -> Result<Device, ServeError> {
        let path = get_device_path(&self.root_path, name);
        load_from_file(&path, "Device").await
    }
    async fn set_device(&self, name: &DeviceName, value: &Device) -> Result<(), ServeError> {
        let path = get_device_path(&self.root_path, name);
        write_to_file(&path, "Device", value).await
    }

    async fn list_devices(&self) -> Result<Vec<DeviceName>, ServeError> {
        list_devices(&self.root_path).await
    }
}

async fn list_devices(root_path: &Path) -> Result<Vec<DeviceName>, ServeError> {
    let search_path = root_path.join("devices");

    let mut devices = vec![];

    if !tokio::fs::try_exists(&search_path)
        .await
        .context("Failed to check device directory existence.")
        .map_err(ServeError::Internal)?
    {
        return Ok(devices);
    }

    let mut dir = tokio::fs::read_dir(&search_path)
        .await
        .context("Failed to read device directory.")
        .map_err(ServeError::Internal)?;

    while let Some(entry) = dir
        .next_entry()
        .await
        .context("Failed to read next device in device directory.")
        .map_err(ServeError::Internal)?
    {
        let path = entry.path();
        if path.is_file() {
            // get file name without extension
            if let Some(file_stem) = path.file_stem() {
                let maybe_device_name = DeviceName::from_str(file_stem.to_string_lossy().as_ref());
                match maybe_device_name {
                    Ok(device_name) => devices.push(device_name),
                    Err(e) => {
                        warn!(
                            "Failed to parse device name from file name: {:?}.\nError: {:?}",
                            path, e
                        );
                    }
                }
            } else {
                warn!("Failed to get file stem from device file path: {:?}", path);
            }
        }
    }

    Ok(devices)
}

async fn load_from_file<T: DeserializeOwned>(
    path: &Path,
    type_name: &str,
) -> Result<T, ServeError> {
    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ServeError::UserFacing(
                StatusCode::NOT_FOUND,
                format!("Failed to find Slipway {type_name} {path:?}."),
            )
        } else {
            ServeError::Internal(anyhow::anyhow!(
                "Failed to load Slipway {type_name} \"{path:?}\".\n{e}",
            ))
        }
    })?;

    let result: T = serde_json::from_slice(&bytes)
        .context(format!(
            "Failed to parse Slipway {type_name} \"{path:?}\" as JSON.",
        ))
        .map_err(ServeError::Internal)?;

    Ok(result)
}

pub(crate) async fn write_to_file<T: Serialize>(
    path: &Path,
    type_name: &str,
    value: &T,
) -> Result<(), ServeError> {
    let bytes = serde_json::to_vec_pretty(value).expect("Device should serialize to JSON");

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            ServeError::Internal(anyhow::anyhow!(
                "Failed to create directory for {type_name} \"{path:?}\".\n{e}",
            ))
        })?;
    }

    tokio::fs::write(&path, &bytes).await.map_err(|e| {
        ServeError::Internal(anyhow::anyhow!(
            "Failed to save Slipway {type_name} \"{path:?}\".\n{e}",
        ))
    })?;
    Ok(())
}

pub(crate) const RIG_FOLDER_NAME: &str = "rigs";
pub(crate) const PLAYLIST_FOLDER_NAME: &str = "playlists";
pub(crate) const DEVICE_FOLDER_NAME: &str = "devices";

pub fn get_rig_folder_path(root_path: &Path) -> PathBuf {
    root_path.join(RIG_FOLDER_NAME)
}

pub fn get_playlist_folder_path(root_path: &Path) -> PathBuf {
    root_path.join(PLAYLIST_FOLDER_NAME)
}

pub fn get_device_folder_path(root_path: &Path) -> PathBuf {
    root_path.join(DEVICE_FOLDER_NAME)
}

fn get_rig_path(root_path: &Path, name: &RigName) -> PathBuf {
    get_rig_folder_path(root_path).join(format!("{}.json", name))
}

fn get_playlist_path(root_path: &Path, name: &PlaylistName) -> PathBuf {
    get_playlist_folder_path(root_path).join(format!("{}.json", name))
}

fn get_device_path(root_path: &Path, name: &DeviceName) -> PathBuf {
    get_device_folder_path(root_path).join(format!("{}.json", name))
}
