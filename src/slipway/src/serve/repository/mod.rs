use std::collections::HashSet;

use actix_web::http::StatusCode;
use async_trait::async_trait;
use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use slipway_host::hash_string;

use crate::primitives::{DeviceName, PlaylistName, RigName};

use super::responses::ServeError;

pub(super) mod file_system;
pub(super) mod memory;

#[async_trait(?Send)]
pub(super) trait ServeRepository: std::fmt::Debug {
    async fn get_rig(&self, name: &RigName) -> Result<slipway_engine::Rig, ServeError>;
    async fn try_get_rig(&self, name: &RigName) -> Result<Option<slipway_engine::Rig>, ServeError> {
        try_load(self.get_rig(name).await)
    }
    async fn set_rig(&self, name: &RigName, value: &slipway_engine::Rig) -> Result<(), ServeError>;
    async fn list_rigs(&self) -> Result<Vec<RigName>, ServeError>;

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError>;
    async fn try_get_playlist(&self, name: &PlaylistName) -> Result<Option<Playlist>, ServeError> {
        try_load(self.get_playlist(name).await)
    }
    async fn set_playlist(&self, name: &PlaylistName, value: &Playlist) -> Result<(), ServeError>;
    async fn list_playlists(&self) -> Result<Vec<PlaylistName>, ServeError>;

    async fn get_device(&self, name: &DeviceName) -> Result<Device, ServeError>;
    async fn try_get_device(&self, name: &DeviceName) -> Result<Option<Device>, ServeError> {
        try_load(self.get_device(name).await)
    }
    async fn set_device(&self, name: &DeviceName, value: &Device) -> Result<(), ServeError>;
    async fn list_devices(&self) -> Result<Vec<DeviceName>, ServeError>;

    async fn get_device_by_id(&self, id: &str) -> Result<(DeviceName, Device), ServeError> {
        for device_name in self.list_devices().await? {
            let device = self.get_device(&device_name).await?;
            if let Some(trmnl_device) = &device.trmnl {
                if trmnl_device.id == id {
                    return Ok((device_name, device));
                }
            }
        }

        Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            format!("Failed to find device with ID {id}."),
        ))
    }
    async fn try_get_device_by_id(
        &self,
        id: &str,
    ) -> Result<Option<(DeviceName, Device)>, ServeError> {
        try_load(self.get_device_by_id(id).await)
    }
    async fn get_device_by_api_key(
        &self,
        api_key: &str,
    ) -> Result<(DeviceName, Device), ServeError> {
        let hashed_api_key = hash_string(api_key);
        for device_name in self.list_devices().await? {
            let device = self.get_device(&device_name).await?;
            if let Some(trmnl_device) = &device.trmnl {
                if trmnl_device.hashed_api_key == hashed_api_key {
                    return Ok((device_name, device));
                }
            }
        }

        Err(ServeError::UserFacing(
            StatusCode::NOT_FOUND,
            "Failed to find device for the given API key.".to_string(),
        ))
    }
}

fn try_load<T>(maybe_result: Result<T, ServeError>) -> Result<Option<T>, ServeError> {
    match maybe_result {
        Ok(result) => Ok(Some(result)),
        Err(ServeError::UserFacing(StatusCode::NOT_FOUND, _)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// A device represents a physical or virtual device which will call the Slipway API.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Device {
    // Any Trmnl API specific settings for the device.
    #[serde(default)]
    pub trmnl: Option<TrmnlDevice>,

    /// The playlist to use for this device.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<PlaylistName>,

    /// The context is included in the Rig data when a Rig is run, and can therefore
    /// be passed to components to affect behavior.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Data specific to devices which make use of the TRMNL API.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct TrmnlDevice {
    /// When the device calls the TRMNL API, this is the value it uses for the ID header.
    /// It is usually the MAC address of the device.
    pub id: String,

    /// The hash of the API key given to the device during setup.
    pub hashed_api_key: String,

    /// If set to true, then a header value is returned to the device from the `display` API
    /// indicating it should reset its firmware.
    #[serde(default)]
    pub reset_firmware: bool,
}

/// A playlist is a collection of Rigs which are run on a device
/// along with information on when to run them.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Playlist {
    pub schedule: Vec<PlaylistItem>,
}

/// A Rig to run on a device along with information on when to run it as part
/// of a playlist.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct PlaylistItem {
    /// The time span during a day this playlist item should be run.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<PlaylistTimeSpan>,

    /// The days of the week this playlist item should be run.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<HashSet<Weekday>>,

    /// When the device should next call the API to update its display after this
    /// playlist item is run.
    pub refresh: Refresh,

    /// The Rig to run.
    pub rig: RigName,
}

/// When the device should next call the API to update its display.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged, rename_all = "snake_case")]
pub(super) enum Refresh {
    Seconds { seconds: u32 },
    Minutes { minutes: u32 },
    Hours { hours: u32 },
    Cron { cron: String },
}

/// A time span for when a Rig should be run as part of a playlist.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged, rename_all = "snake_case")]
pub(super) enum PlaylistTimeSpan {
    From { from: NaiveTime },
    To { to: NaiveTime },
    Between { from: NaiveTime, to: NaiveTime },
}
