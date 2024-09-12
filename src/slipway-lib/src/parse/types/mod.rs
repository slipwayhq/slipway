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

use std::{collections::HashMap, sync::Arc};

use jsonschema::JSONSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::errors::{AppError, ComponentLoadError};

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

fn parse_component_version(version_string: &str) -> Result<Version, AppError> {
    Version::parse(version_string).map_err(|e| AppError::InvalidSlipwayPrimitive {
        primitive_type: stringify!(Version).to_string(),
        message: e.to_string(),
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
pub struct Component<TSchema> {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
    pub description: Option<Description>,
    pub input: TSchema,
    pub output: TSchema,
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
        schema: jsonschema::JSONSchema,
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
                schema: jsonschema::JSONSchema::compile(original)
                    .expect("cloned schema should be valid"),
                original: original.clone(),
            },
        }
    }
}

pub fn parse_schema(
    schema_name: &str,
    schema: serde_json::Value,
) -> Result<Schema, ComponentLoadError> {
    if let Some(serde_json::Value::String(schema_uri)) = schema.get("$schema") {
        if schema_uri.contains("://json-schema.org/") {
            // If the schema contains a $schema property, and the domain is json-schema.org, it is a JSON Schema.
            let compiled_schema = JSONSchema::compile(&schema).map_err(|e| {
                ComponentLoadError::JsonSchemaParseFailed {
                    schema_name: schema_name.to_string(),
                    error: e.into(),
                }
            })?;

            return Ok(Schema::JsonSchema {
                schema: compiled_schema,
                original: schema,
            });
        }
    }

    // Otherwise it is JsonTypeDef.
    let jtd_serde_schema: jtd::SerdeSchema =
        serde_json::from_value(schema).map_err(|e| ComponentLoadError::JsonTypeDefParseFailed {
            schema_name: schema_name.to_string(),
            error: Arc::new(e),
        })?;

    let jtd_schema = jtd::Schema::from_serde_schema(jtd_serde_schema)?;

    Ok(Schema::JsonTypeDef { schema: jtd_schema })
}
