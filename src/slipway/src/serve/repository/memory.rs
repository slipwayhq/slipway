use std::collections::HashMap;

use actix_web::http::StatusCode;
use async_trait::async_trait;

use crate::{
    primitives::{DeviceName, PlaylistName, RigName},
    serve::responses::ServeError,
};

use super::{Device, Playlist, ServeRepository};

#[derive(Clone, Debug)]
pub(crate) struct MemoryRepository {
    devices: HashMap<DeviceName, Device>,
    playlists: HashMap<PlaylistName, Playlist>,
    rigs: HashMap<RigName, slipway_engine::Rig>,
}

impl MemoryRepository {
    pub fn new(
        devices: HashMap<DeviceName, Device>,
        playlists: HashMap<PlaylistName, Playlist>,
        rigs: HashMap<RigName, slipway_engine::Rig>,
    ) -> Self {
        Self {
            devices,
            playlists,
            rigs,
        }
    }
}

#[async_trait(?Send)]
impl ServeRepository for MemoryRepository {
    async fn get_rig(&self, name: &RigName) -> Result<slipway_engine::Rig, ServeError> {
        self.rigs.get(name).cloned().ok_or_else(|| {
            ServeError::UserFacing(StatusCode::NOT_FOUND, format!("Rig not found: {}", name))
        })
    }

    async fn set_rig(
        &self,
        _name: &RigName,
        _value: &slipway_engine::Rig,
    ) -> Result<(), ServeError> {
        unimplemented!();
    }

    async fn list_rigs(&self) -> Result<Vec<RigName>, ServeError> {
        Ok(self.rigs.keys().cloned().collect())
    }

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError> {
        self.playlists.get(name).cloned().ok_or_else(|| {
            ServeError::UserFacing(
                StatusCode::NOT_FOUND,
                format!("Playlist not found: {}", name),
            )
        })
    }

    async fn set_playlist(
        &self,
        _name: &PlaylistName,
        _value: &Playlist,
    ) -> Result<(), ServeError> {
        unimplemented!();
    }

    async fn list_playlists(&self) -> Result<Vec<PlaylistName>, ServeError> {
        Ok(self.playlists.keys().cloned().collect())
    }

    async fn get_device(&self, name: &DeviceName) -> Result<Device, ServeError> {
        self.devices.get(name).cloned().ok_or_else(|| {
            ServeError::UserFacing(StatusCode::NOT_FOUND, format!("Device not found: {}", name))
        })
    }
    async fn set_device(&self, _name: &DeviceName, _value: &Device) -> Result<(), ServeError> {
        unimplemented!();
    }

    async fn list_devices(&self) -> Result<Vec<DeviceName>, ServeError> {
        Ok(self.devices.keys().cloned().collect())
    }
}
