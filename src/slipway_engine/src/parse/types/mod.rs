//! `Rig` and `Component` do not use a `SlipwayId` type
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

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::errors::RigError;

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

fn parse_component_version(version_string: &str) -> Result<Version, RigError> {
    Version::parse(version_string).map_err(|e| RigError::InvalidSlipwayPrimitive {
        primitive_type: stringify!(Version).to_string(),
        message: e.to_string(),
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rig {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
    pub description: Option<Description>,
    pub constants: Option<serde_json::Value>,
    pub rigging: Rigging,
}

impl Rig {
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
    pub callouts: Option<HashMap<ComponentHandle, ComponentCallout>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ComponentPermission {
    Url { url: String },
    Domain { domain: String },
    UrlRegex { regex: String },

    File { path: String },
    Directory { path: String },
    FileRegex { regex: String },

    Env { value: String },
    EnvRegex { regex: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComponentCallout {
    pub component: SlipwayReference,
    pub callouts: Option<HashMap<ComponentHandle, ComponentCallout>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Component<TSchema> {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
    pub description: Option<Description>,
    pub input: TSchema,
    pub output: TSchema,
    pub constants: Option<serde_json::Value>,
    pub rigging: Option<Rigging>,
    pub callouts: Option<HashMap<ComponentHandle, ComponentCallout>>,
}

impl<TSchema> Component<TSchema> {
    pub fn get_id(&self) -> SlipwayId {
        SlipwayId::new(&self.publisher, &self.name, &self.version)
    }
}

#[derive(Debug)]
pub enum Schema {
    JsonTypeDef {
        schema: jtd::Schema,
    },
    JsonSchema {
        schema: jsonschema::Validator,
        original: serde_json::Value,
    },
}

impl Serialize for Schema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Schema::JsonTypeDef { schema } => {
                schema.clone().into_serde_schema().serialize(serializer)
            }
            Schema::JsonSchema {
                schema: _,
                original,
            } => original.serialize(serializer),
        }
    }
}

impl Clone for Schema {
    fn clone(&self) -> Self {
        match self {
            Schema::JsonTypeDef { schema } => Schema::JsonTypeDef {
                schema: schema.clone(),
            },
            Schema::JsonSchema {
                schema: _,
                original,
            } => Schema::JsonSchema {
                schema: jsonschema::Validator::options()
                    .build(original)
                    .expect("cloned schema should be valid"),
                original: original.clone(),
            },
        }
    }
}
