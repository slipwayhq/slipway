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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,

    #[serde(skip_serializing_if = "Option::is_none")]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<Permission>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<Permission>>,

    #[serde(skip_serializing_if = "Option::is_none")]
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
#[serde(tag = "permission", rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(untagged, rename_all = "snake_case", deny_unknown_fields)]
pub enum StringPermission {
    Any {},
    Exact { exact: String },
    Prefix { prefix: String },
    Suffix { suffix: String },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case", deny_unknown_fields)]
pub enum PathPermission {
    Any {},
    Exact { exact: PathBuf },
    Within { within: PathBuf },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalComponentPermission {
    Any {},

    // We only support exact for component paths because we
    // don't know where the ComponentsLoader implementation will load
    // the components from, which makes it hard to check paths using
    // PathPermission and canonicalize.
    Exact { exact: String },
}

/// The structure of this enum is designed to allow user friendly JSON.
/// See `Permission`` for more details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case", deny_unknown_fields)]
pub enum UrlPermission {
    Any {},
    Exact { exact: Url },
    Prefix { prefix: Url },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RegistryComponentPermission {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<semver::VersionReq>,
}

pub(crate) static PERMISSIONS_ALL_VEC: LazyLock<Vec<Permission>> =
    LazyLock::new(|| vec![Permission::All]);

pub(crate) static PERMISSIONS_NONE_VEC: LazyLock<Vec<Permission>> = LazyLock::new(Vec::new);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Component<TSchema> {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,

    pub input: TSchema,
    pub output: TSchema,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub constants: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rigging: Option<Rigging>,

    #[serde(skip_serializing_if = "Option::is_none")]
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

#[cfg(test)]
mod tests {
    use common_macros::slipway_test;
    use semver::VersionReq;

    use super::*;

    #[slipway_test]
    fn test_deserialize_all_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"all"}"#).unwrap(),
            Permission::All
        );
    }

    #[slipway_test]
    fn test_deserialize_http_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"http"}"#).unwrap(),
            Permission::Http(UrlPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http", "exact": "https://example.com/foo.txt"}"#
            )
            .unwrap(),
            Permission::Http(UrlPermission::Exact {
                exact: Url::parse("https://example.com/foo.txt").unwrap()
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http", "prefix": "https://example.com/foo/"}"#
            )
            .unwrap(),
            Permission::Http(UrlPermission::Prefix {
                prefix: Url::parse("https://example.com/foo/").unwrap()
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_file_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"file"}"#).unwrap(),
            Permission::File(PathPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"file", "exact": "./foo.txt"}"#)
                .unwrap(),
            Permission::File(PathPermission::Exact {
                exact: PathBuf::from("./foo.txt")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"file", "within": "./foo"}"#)
                .unwrap(),
            Permission::File(PathPermission::Within {
                within: PathBuf::from("./foo")
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_font_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"font"}"#).unwrap(),
            Permission::Font(StringPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"font", "exact": "foo"}"#).unwrap(),
            Permission::Font(StringPermission::Exact {
                exact: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"font", "prefix": "foo"}"#)
                .unwrap(),
            Permission::Font(StringPermission::Prefix {
                prefix: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"font", "suffix": "foo"}"#)
                .unwrap(),
            Permission::Font(StringPermission::Suffix {
                suffix: String::from("foo")
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_env_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"env"}"#).unwrap(),
            Permission::Env(StringPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"env", "exact": "foo"}"#).unwrap(),
            Permission::Env(StringPermission::Exact {
                exact: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"env", "prefix": "foo"}"#).unwrap(),
            Permission::Env(StringPermission::Prefix {
                prefix: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"env", "suffix": "foo"}"#).unwrap(),
            Permission::Env(StringPermission::Suffix {
                suffix: String::from("foo")
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_registry_component_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"registry_component"}"#).unwrap(),
            Permission::RegistryComponent(RegistryComponentPermission {
                publisher: None,
                name: None,
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_component", "publisher": "foo"}"#
            )
            .unwrap(),
            Permission::RegistryComponent(RegistryComponentPermission {
                publisher: Some("foo".into()),
                name: None,
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_component", "name": "foo"}"#
            )
            .unwrap(),
            Permission::RegistryComponent(RegistryComponentPermission {
                publisher: None,
                name: Some("foo".into()),
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_component", "version": "1.0.0"}"#
            )
            .unwrap(),
            Permission::RegistryComponent(RegistryComponentPermission {
                publisher: None,
                name: None,
                version: Some(VersionReq::parse("1.0.0").unwrap())
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_component", "publisher": "foo", "name": "bar", "version": "1.2.3"}"#
            )
            .unwrap(),
            Permission::RegistryComponent(RegistryComponentPermission {
                publisher: Some("foo".into()),
                name: Some("bar".into()),
                version: Some(VersionReq::parse("1.2.3").unwrap())
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_http_component_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"http_component"}"#).unwrap(),
            Permission::HttpComponent(UrlPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http_component", "exact": "https://example.com/foo.txt"}"#
            )
            .unwrap(),
            Permission::HttpComponent(UrlPermission::Exact {
                exact: Url::parse("https://example.com/foo.txt").unwrap()
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http_component", "prefix": "https://example.com/foo/"}"#
            )
            .unwrap(),
            Permission::HttpComponent(UrlPermission::Prefix {
                prefix: Url::parse("https://example.com/foo/").unwrap()
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_local_component_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"local_component"}"#).unwrap(),
            Permission::LocalComponent(LocalComponentPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"local_component", "exact": "foo"}"#
            )
            .unwrap(),
            Permission::LocalComponent(LocalComponentPermission::Exact {
                exact: String::from("foo")
            })
        );
    }
}
