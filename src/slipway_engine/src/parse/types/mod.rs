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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::errors::RigError;

use self::{
    primitives::{ComponentHandle, Description, Name, Publisher},
    slipway_id::SlipwayId,
    slipway_reference::SlipwayReference,
};

use super::url::{process_url_str, ProcessedUrl};

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

pub type Callouts = HashMap<ComponentHandle, SlipwayReference>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComponentRigging {
    pub component: SlipwayReference,
    pub input: Option<serde_json::Value>,
    pub allow: Option<Vec<Permission>>,
    pub deny: Option<Vec<Permission>>,
    pub callouts: Option<Callouts>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "permission", rename_all = "snake_case")]
pub enum Permission {
    All,

    HttpFetch(UrlPermission),

    FontQuery(StringPermission),

    RegistryComponent(StringPermission),
    HttpComponent(UrlPermission),
    FileComponent(PathPermission),
}

impl Permission {
    pub fn all() -> Vec<Permission> {
        vec![Permission::All]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "variant", content = "value", rename_all = "snake_case")]
pub enum StringPermission {
    Any,
    Exact(String),
    Prefix(String),
    Suffix(String),
}

impl StringPermission {
    pub fn matches(&self, string: &str) -> bool {
        match self {
            StringPermission::Any => true,
            StringPermission::Exact(value) => value == string,
            StringPermission::Prefix(value) => string.starts_with(value),
            StringPermission::Suffix(value) => string.ends_with(value),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "variant", content = "value", rename_all = "snake_case")]
pub enum PathPermission {
    Any,
    Exact(PathBuf),
    Within(PathBuf),
}

impl PathPermission {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            PathPermission::Any => true,
            PathPermission::Exact(value) => match (value.canonicalize(), path.canonicalize()) {
                (Ok(val), Ok(p)) => val == p,
                _ => false,
            },
            PathPermission::Within(value) => match (value.canonicalize(), path.canonicalize()) {
                (Ok(val), Ok(p)) => p.starts_with(&val),
                _ => false,
            },
        }
    }

    pub fn matches_url_str(&self, url_str: &str) -> bool {
        let Ok(processed_url) = process_url_str(url_str) else {
            return false;
        };

        match processed_url {
            ProcessedUrl::RelativePath(path) => self.matches(&path),
            ProcessedUrl::AbsolutePath(path) => self.matches(&path),
            ProcessedUrl::Url(_) => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "variant", content = "value", rename_all = "snake_case")]
pub enum UrlPermission {
    Any,
    Exact(Url),
    Prefix(Url),
}

impl UrlPermission {
    pub fn matches(&self, url: &Url) -> bool {
        match self {
            UrlPermission::Any => true,
            UrlPermission::Exact(value) => value.as_str() == url.as_str(),
            UrlPermission::Prefix(value) => url.as_str().starts_with(value.as_str()),
        }
    }
}

pub(crate) static PERMISSIONS_ALL_VEC: Lazy<Vec<Permission>> = Lazy::new(|| vec![Permission::All]);

pub(crate) static PERMISSIONS_NONE_VEC: Lazy<Vec<Permission>> = Lazy::new(Vec::new);

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
    pub callouts: Option<Callouts>,
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
        schema: Box<jsonschema::Validator>, // Boxed to keep enum variant size similar.
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
                schema: Box::new(
                    jsonschema::Validator::options()
                        .build(original)
                        .expect("cloned schema should be valid"),
                ),
                original: original.clone(),
            },
        }
    }
}
