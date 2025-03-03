use actix_web::http::StatusCode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::primitives::{DeviceName, PlaylistName, RigName};

use super::ServeError;

pub(super) mod file_system;

#[async_trait(?Send)]
pub(super) trait ServeRepository {
    async fn get_rig(&self, name: &RigName) -> Result<slipway_engine::Rig, ServeError>;
    async fn try_get_rig(&self, name: &RigName) -> Result<Option<slipway_engine::Rig>, ServeError> {
        try_load(self.get_rig(name).await)
    }
    async fn set_rig(&self, name: &RigName, value: &slipway_engine::Rig) -> Result<(), ServeError>;

    async fn get_device(&self, id: &str) -> Result<Device, ServeError>;
    async fn try_get_device(&self, id: &str) -> Result<Option<Device>, ServeError> {
        try_load(self.get_device(id).await)
    }
    async fn set_device(&self, id: &str, value: &Device) -> Result<(), ServeError>;

    async fn get_playlist(&self, name: &PlaylistName) -> Result<Playlist, ServeError>;
    async fn try_get_playlist(&self, name: &PlaylistName) -> Result<Option<Playlist>, ServeError> {
        try_load(self.get_playlist(name).await)
    }
    async fn set_playlist(&self, name: &PlaylistName, value: &Playlist) -> Result<(), ServeError>;
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
    pub friendly_id: String,
    pub hashed_api_key: String,
    pub name: DeviceName,
    pub playlist: Option<PlaylistName>,
    pub context: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Playlist {
    pub items: Vec<PlaylistItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct PlaylistItem {
    #[serde(flatten)]
    pub span: Option<PlaylistTimeSpan>,
    pub days: Option<Vec<PlaylistDay>>,

    /// The interval to send back to the device for the next update.
    /// The plan is to allow this to be overridden by Rigs.
    pub interval_seconds: u32,
    pub rig: RigName,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub(super) enum PlaylistTimeSpan {
    From { from: String },
    To { to: String },
    Between { from: String, to: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlaylistDay {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}
