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

pub(crate) const TEST_PUBLISHER: &str = "test_publisher";

fn parse_component_version(version_string: &str) -> Result<Version, SlipwayError> {
    Version::parse(version_string).map_err(|e| {
        SlipwayError::InvalidSlipwayPrimitive(stringify!(Version).to_string(), e.to_string())
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct App {
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
pub struct Rigging {
    #[serde(flatten)]
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub components: HashMap<ComponentHandle, ComponentRigging>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComponentRigging {
    pub component: SlipwayReference,
    pub input: Option<serde_json::Value>,
    pub permissions: Option<Vec<ComponentPermission>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ComponentPermission {
    Url { url: String },
    Domain { domain: String },
    UrlRegex { regex: String },

    File { path: String },
    Folder { path: String },
    FileRegex { regex: String },

    Env { value: String },
    EnvRegex { regex: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Component {
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

#[cfg(feature = "internal")]
mod tests {
    use super::*;
    use crate::utils::ch;
    use serde_json::json;
    use serde_json::Value;
    use std::str::FromStr;

    impl App {
        pub fn for_test(rigging: Rigging) -> App {
            App {
                publisher: Publisher::from_str(TEST_PUBLISHER).unwrap(),
                name: Name::from_str("test_name").unwrap(),
                version: Version::from_str("0.1.0").unwrap(),
                description: None,
                constants: Some(json!({"test_constant": "test_constant_value"})),
                rigging,
            }
        }
    }

    impl ComponentRigging {
        pub fn for_test(name: &str, input: Option<Value>) -> (ComponentHandle, ComponentRigging) {
            (
                ch(name),
                ComponentRigging {
                    component: SlipwayReference::from_str(&format!("p{name}.{name}.0.1.0"))
                        .unwrap(),
                    input,
                    permissions: None,
                },
            )
        }
    }
}
