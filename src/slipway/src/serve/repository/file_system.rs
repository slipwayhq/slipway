use std::path::{Path, PathBuf};

use actix_web::http::StatusCode;
use anyhow::Context;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    primitives::{PlaylistName, RigName},
    serve::ServeError,
};

use super::{Device, Playlist, ServeRepository};

#[derive(Clone)]
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

    async fn get_device(&self, id: &str) -> Result<Device, ServeError> {
        let path = get_device_path(&self.root_path, id);
        load_from_file(&path, "Device").await
    }
    async fn set_device(&self, id: &str, value: &Device) -> Result<(), ServeError> {
        let path = get_device_path(&self.root_path, id);
        write_to_file(&path, "Device", value).await
    }

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError> {
        let path = get_playlist_path(&self.root_path, name);
        load_from_file(&path, "Playlist").await
    }

    async fn set_playlist(&self, name: &PlaylistName, value: &Playlist) -> Result<(), ServeError> {
        let path = get_playlist_path(&self.root_path, name);
        write_to_file(&path, "Playlist", value).await
    }

    // async fn create_device(
    //     &self,
    //     id: &str,
    //     friendly_id: &str,
    //     api_key: &str,
    // ) -> Result<Option<Device>, ServeError> {
    //     let device = Device {
    //         friendly_id: friendly_id.to_string(),
    //         api_key: api_key.to_string(),
    //         name: "<name_of_device>".to_string(),
    //         playlist: "<desired_playlist>".to_string(),
    //         context: serde_json::json!({
    //             "description": "The context will be passed into the rigs.",
    //         }),
    //     };

    //     let path = get_device_path(&self.root_path, id);

    //     warn!("A request to create a device was received.");
    //     warn!("The device has the ID \"{id}\".");
    //     warn!("If you wish to allow this device, create the following file:");
    //     warn!("{path:?}");
    //     warn!(
    //         "Suggested initial file content:\n{}",
    //         serde_json::to_string_pretty(&device).expect("Device should serialize to JSON")
    //     );
    //     warn!("Don't forget to re-deploy if necessary.");

    //     Ok(None)
    // }
}

// pub(crate) async fn write_empty_playlist_if_not_exist(
//     root_path: &Path,
//     name: &str,
// ) -> anyhow::Result<()> {
//     let path = get_playlist_path(root_path, name);

//     // Check if file exists using tokio
//     if tokio::fs::metadata(&path).await.is_ok() {
//         return Ok(());
//     }

//     let playlist = Playlist { items: vec![] };

//     let bytes = serde_json::to_vec_pretty(&playlist).expect("Playlist should serialize to JSON");
//     tokio::fs::write(&path, &bytes).await?;
//     Ok(())
// }

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

async fn write_to_file<T: Serialize>(
    path: &Path,
    type_name: &str,
    value: &T,
) -> Result<(), ServeError> {
    let bytes = serde_json::to_vec_pretty(value).expect("Device should serialize to JSON");
    tokio::fs::write(&path, &bytes).await.map_err(|e| {
        ServeError::Internal(anyhow::anyhow!(
            "Failed to save Slipway {type_name} \"{path:?}\".\n{e}",
        ))
    })?;
    Ok(())
}

fn get_rig_path(root_path: &Path, name: &RigName) -> PathBuf {
    root_path.join(format!("rig/{}.json", name))
}

fn get_playlist_path(root_path: &Path, name: &PlaylistName) -> PathBuf {
    root_path.join(format!("playlist/{}.json", name))
}

fn get_device_path(root_path: &Path, id: &str) -> PathBuf {
    // Replace any characters that are not alphanumeric or a dash with an underscore.
    let file_safe_id = id
        .replace(|c: char| !c.is_alphanumeric(), "_")
        .to_lowercase();
    root_path.join(format!("device/{}.json", file_safe_id))
}
