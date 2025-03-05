use std::collections::HashSet;

use actix_web::http::StatusCode;
use async_trait::async_trait;
use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

use crate::primitives::{DeviceName, PlaylistName, RigName};

use super::ServeError;

pub(super) mod file_system;

#[async_trait(?Send)]
pub(super) trait ServeRepository: std::fmt::Debug {
    async fn get_rig(&self, name: &RigName) -> Result<slipway_engine::Rig, ServeError>;
    async fn try_get_rig(&self, name: &RigName) -> Result<Option<slipway_engine::Rig>, ServeError> {
        try_load(self.get_rig(name).await)
    }
    async fn set_rig(&self, name: &RigName, value: &slipway_engine::Rig) -> Result<(), ServeError>;

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError>;
    async fn try_get_playlist(&self, name: &PlaylistName) -> Result<Option<Playlist>, ServeError> {
        try_load(self.get_playlist(name).await)
    }
    async fn set_playlist(&self, name: &PlaylistName, value: &Playlist) -> Result<(), ServeError>;

    async fn get_device(&self, name: &DeviceName) -> Result<Device, ServeError>;
    async fn try_get_device(&self, name: &DeviceName) -> Result<Option<Device>, ServeError> {
        try_load(self.get_device(name).await)
    }
    async fn set_device(&self, name: &DeviceName, value: &Device) -> Result<(), ServeError>;

    async fn list_devices(&self) -> Result<Vec<DeviceName>, ServeError>;
    async fn get_device_by_id(&self, id: &str) -> Result<Device, ServeError> {
        for device_name in self.list_devices().await? {
            let device = self.get_device(&device_name).await?;
            if device.id == id {
                return Ok(device);
            }
        }

        Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            format!("Failed to find device with ID {id}."),
        ))
    }
    async fn try_get_device_by_id(&self, id: &str) -> Result<Option<Device>, ServeError> {
        try_load(self.get_device_by_id(id).await)
    }
}

fn try_load<T>(maybe_result: Result<T, ServeError>) -> Result<Option<T>, ServeError> {
    match maybe_result {
        Ok(result) => Ok(Some(result)),
        Err(ServeError::UserFacing(StatusCode::NOT_FOUND, _)) => Ok(None),
        Err(e) => Err(e),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Device {
    pub id: String,
    pub friendly_id: String,
    pub hashed_api_key: String,
    pub name: DeviceName,

    #[serde(default)]
    pub playlist: Option<PlaylistName>,

    pub context: serde_json::Value,

    #[serde(default)]
    pub reset_firmware: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Playlist {
    pub items: Vec<PlaylistItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct PlaylistItem {
    #[serde(flatten)]
    pub span: Option<PlaylistTimeSpan>,
    pub days: Option<HashSet<Weekday>>,
    pub refresh: Refresh,
    pub rig: RigName,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub(super) enum Refresh {
    Seconds { seconds: u32 },
    Minutes { minutes: u32 },
    Hours { hours: u32 },
    Cron { cron: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub(super) enum PlaylistTimeSpan {
    From { from: NaiveTime },
    To { to: NaiveTime },
    Between { from: NaiveTime, to: NaiveTime },
}
