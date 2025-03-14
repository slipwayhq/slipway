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

use std::{collections::HashMap, path::PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use url::Url;

use crate::errors::RigError;

use self::{
    primitives::{ComponentHandle, Description, Name, Publisher},
    slipway_id::SlipwayId,
    slipway_reference::SlipwayReference,
};

mod local_component_permission;
mod path_permission;
pub(crate) mod primitives;
mod registry_component_permission;
pub(crate) mod slipway_id;
pub(crate) mod slipway_reference;
mod string_permission;
mod url_permission;

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
    pub description: Option<Description>,
    pub constants: Option<serde_json::Value>,
    pub rigging: Rigging,
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

/// The structure of this enum is designed to allow user friendly JSON.
/// For example:
/// ```json
/// {
///   "permission": "http"
/// }
/// ```
/// for `Permission::Http(UrlPermission::Any)`.
/// and
/// ```json
/// {
///   "permission": "http",
///   "prefix": "https://example.com/"
/// }
/// ```
/// for `Permission::Http(UrlPermission::Prefix { prefix: "https://example.com/".into() })`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "permission", rename_all = "snake_case")]
pub enum Permission {
    All,

    Http(UrlPermission),

    File(PathPermission),

    Font(StringPermission),

    Env(StringPermission),

    RegistryComponent(RegistryComponentPermission),
    HttpComponent(UrlPermission),
    LocalComponent(LocalComponentPermission),
}

impl Permission {
    pub fn all() -> Vec<Permission> {
        vec![Permission::All]
    }
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum StringPermission {
    Any {},
    Exact { exact: String },
    Prefix { prefix: String },
    Suffix { suffix: String },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum PathPermission {
    Any {},
    Exact { exact: PathBuf },
    Within { within: PathBuf },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum LocalComponentPermission {
    Any,

    // We only support exact for component paths because we
    // don't know where the ComponentsLoader implementation will load
    // the components from, which makes it hard to check paths using
    // PathPermission and canonicalize.
    Exact { exact: String },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum UrlPermission {
    Any {},
    Exact { exact: Url },
    Prefix { prefix: Url },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct RegistryComponentPermission {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<semver::VersionReq>,
}

// impl PathPermission {
//     pub fn matches(&self, path: &Path) -> bool {
//         match self {
//             PathPermission::Any => true,
//             PathPermission::Exact(value) => match (value.canonicalize(), path.canonicalize()) {
//                 (Ok(val), Ok(p)) => val == p,
//                 _ => false,
//             },
//             PathPermission::Within(value) => match (value.canonicalize(), path.canonicalize()) {
//                 (Ok(val), Ok(p)) => p.starts_with(&val),
//                 (Err(value_e), Err(path_e)) => {
//                     warn!(
//                         "Error canonicalizing path to check permissions: {:?}, error: {:?}",
//                         value, value_e
//                     );
//                     warn!(
//                         "Error canonicalizing path to check permissions: {:?}, error: {:?}",
//                         path, path_e
//                     );
//                     false
//                 }
//                 (Err(value_e), Ok(_)) => {
//                     warn!(
//                         "Error canonicalizing path to check permissions: {:?}, error: {:?}",
//                         value, value_e
//                     );
//                     false
//                 }
//                 (Ok(_), Err(path_e)) => {
//                     warn!(
//                         "Error canonicalizing path to check permissions: {:?}, error: {:?}",
//                         path, path_e
//                     );
//                     false
//                 }
//             },
//         }
//     }

//     pub fn matches_url_str(&self, url_str: &str) -> bool {
//         let Ok(processed_url) = process_url_str(url_str) else {
//             return false;
//         };

//         match processed_url {
//             ProcessedUrl::RelativePath(path) => self.matches(&path),
//             ProcessedUrl::AbsolutePath(path) => self.matches(&path),
//             ProcessedUrl::Url(_) => false,
//         }
//     }
// }

pub(crate) static PERMISSIONS_ALL_VEC: LazyLock<Vec<Permission>> =
    LazyLock::new(|| vec![Permission::All]);

pub(crate) static PERMISSIONS_NONE_VEC: LazyLock<Vec<Permission>> = LazyLock::new(Vec::new);

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
