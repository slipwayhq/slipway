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

use self::{
    primitives::{ComponentHandle, Description, Name, Publisher},
    slipway_id::SlipwayId,
    slipway_reference::SlipwayReference,
};

pub(crate) mod primitives;
pub(crate) mod slipway_id;
pub(crate) mod slipway_reference;

pub(crate) const REGISTRY_PUBLISHER_SEPARATOR: char = '.';
pub(crate) const VERSION_SEPARATOR: char = '.';

#[cfg(test)]
pub(crate) const TEST_PUBLISHER: &str = "test_publisher";

fn parse_component_version(version_string: &str) -> Result<Version, SlipwayError> {
    Version::parse(version_string).map_err(|e| {
        SlipwayError::InvalidSlipwayPrimitive(stringify!(Version).to_string(), e.to_string())
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct App {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
    pub description: Option<Description>,
    pub constants: Option<serde_json::Value>,
    pub rigging: Rigging,
}

impl App {
    pub fn get_id(&self) -> SlipwayId {
        SlipwayId::new(&self.publisher, &self.name, &self.version)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rigging {
    #[serde(flatten)]
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub components: HashMap<ComponentHandle, ComponentRigging>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComponentRigging {
    pub component: SlipwayReference,
    pub input: Option<serde_json::Value>,
    pub permissions: Option<ComponentPermissions>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComponentPermissions {
    pub network: Option<String>,
    pub file_system: Option<String>,
    pub environment: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Component {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
    pub description: Option<Description>,
    pub input: SerdeSchema,
    pub output: SerdeSchema,
}

impl Component {
    pub fn get_id(&self) -> SlipwayId {
        SlipwayId::new(&self.publisher, &self.name, &self.version)
    }
}
