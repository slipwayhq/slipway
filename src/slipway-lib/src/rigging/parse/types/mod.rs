//! `App` and `Component` do not use a `SlipwayId` type
//! and instead specify `publisher`, `name`, and `version` separately.
//! This reflects how we want the fields to appear in the JSON.
//!
//! We could use `SlipwayId` and `serde(flatten)` but this then doesn't
//! support `deny_unknown_fields`.
//!
//! Keeping the fields separate and at the root
//! makes tooling simpler (such as auto-incrementing versions),
//! and is what the users will expect based on other toolchains
//! such as Node's package.json.

use std::collections::HashMap;

use jtd::SerdeSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::errors::SlipwayError;

use self::{slipway_id::SlipwayId, slipway_reference::SlipwayReference};

mod slipway_id;
mod slipway_reference;

pub(crate) const REGISTRY_PUBLISHER_SEPARATOR: char = '.';
pub(crate) const VERSION_SEPARATOR: char = '#';

#[cfg(test)]
pub(crate) const TEST_PUBLISHER: &str = "test-publisher";

fn parse_component_version(version_string: &str) -> Result<Version, SlipwayError> {
    Version::parse(version_string).map_err(|e| SlipwayError::InvalidSlipwayReference(e.to_string()))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    pub publisher: String,
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub constants: Option<serde_json::Value>,
    pub rigging: Rigging,
}

impl App {
    pub fn get_id(&self) -> SlipwayId {
        SlipwayId::new(&self.publisher, &self.name, &self.version)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Rigging {
    #[serde(flatten)]
    pub components: HashMap<String, ComponentRigging>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentRigging {
    pub component: SlipwayReference,
    pub input: Option<serde_json::Value>,
    pub permissions: Option<ComponentPermissions>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentPermissions {
    pub network: Option<String>,
    pub file_system: Option<String>,
    pub environment: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub publisher: String,
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub input: SerdeSchema,
    pub output: SerdeSchema,
}

impl Component {
    pub fn get_id(&self) -> SlipwayId {
        SlipwayId::new(&self.publisher, &self.name, &self.version)
    }
}
