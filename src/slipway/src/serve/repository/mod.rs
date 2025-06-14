use std::collections::HashSet;

use actix_web::http::StatusCode;
use async_trait::async_trait;
use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

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
    /// The playlist to use for this device.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist: Option<PlaylistName>,

    /// The context is included in the Rig data when a Rig is run, and can therefore
    /// be passed to components to affect behavior.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,

    #[serde(flatten)]
    pub result_spec: RigResultPartialSpec,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct RigResultPartialSpec {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<RigResultFormat>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_format: Option<RigResultImageFormat>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<u16>,
}

impl RigResultPartialSpec {
    pub fn into_spec(self, defaults: RigResultSpec) -> RigResultSpec {
        RigResultSpec {
            format: self.format.unwrap_or(defaults.format),
            image_format: self.image_format.unwrap_or(defaults.image_format),
            rotate: self.rotate.unwrap_or(defaults.rotate),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct RigResultSpec {
    #[serde(default)]
    pub format: RigResultFormat,

    #[serde(default)]
    pub image_format: RigResultImageFormat,

    #[serde(default)]
    pub rotate: u16,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub(super) enum RigResultImageFormat {
    Jpeg,

    #[default]
    Png,

    // Specify serde string as "bmp_1bit", to avoid default of `bmp1_bit`.
    #[serde(rename = "bmp_1bit")]
    Bmp1Bit,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub(super) enum RigResultFormat {
    /// Return an image.
    #[default]
    Image,

    /// Return the JSON output of the rig.
    Json,

    /// Return the image encoded as a data URL.
    /// We expose this as `html` to the user because while internally this is a data URL,
    /// the user sees it as an HTML page containing a data URL.
    #[serde(rename = "html")]
    DataUrl,

    /// Return a URL which will generate the image.
    /// We expose this as `html_js` to the user because while internally this is a URL,
    /// the user sees it as an HTML page containing a URL.
    /// Eventually we'll use JS to refresh the image without reloading the page, hence the
    /// name.
    #[serde(rename = "html_js")]
    Url,
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
