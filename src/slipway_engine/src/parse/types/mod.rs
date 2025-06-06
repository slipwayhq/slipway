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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<DefaultRigContext>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DefaultRigContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Rigging {
    #[serde(flatten)]
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub components: HashMap<ComponentHandle, ComponentRigging>,
}

pub type Callouts = HashMap<ComponentHandle, Callout>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callout {
    pub component: SlipwayReference,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<Permission>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<Permission>>,
}

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

    // This is used for outputting the debug rig. We need to keep the entire
    // permissions chain for each component, so we can't just use `allow` and `deny`.
    // Originally we had an enum which either had allow/deny fields above,
    // or a permissions_chain field. We used `flatten` to make this transparent
    // to the user, but `flatten` isn't compatible with `deny_unknown_fields`,
    // and losing `deny_unknown_fields` would make the experience worse for the user
    // so we decided to settle on having all three properties in `ComponentRigging`
    // with a runtime check to ensure only one is used at a time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions_chain: Option<Vec<PermissionsChainLink>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub callouts: Option<Callouts>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PermissionsChainLink {
    pub allow: Vec<Permission>,
    pub deny: Vec<Permission>,
}

pub struct PermissionsChainLinkRef<'a> {
    pub allow: &'a Vec<Permission>,
    pub deny: &'a Vec<Permission>,
}

impl ComponentRigging {
    pub fn permissions_as_chain(&self) -> Vec<PermissionsChainLinkRef> {
        if let Some(permissions_chain) = self.permissions_chain.as_ref() {
            if self.allow.is_some() || self.deny.is_some() {
                panic!(
                    "ComponentRigging should have either allow/deny or permissions_chain, not both"
                );
            }

            if permissions_chain.is_empty() {
                vec![PermissionsChainLinkRef {
                    allow: &PERMISSIONS_NONE_VEC,
                    deny: &PERMISSIONS_NONE_VEC,
                }]
            } else {
                permissions_chain
                    .iter()
                    .map(|item| PermissionsChainLinkRef {
                        allow: &item.allow,
                        deny: &item.deny,
                    })
                    .collect()
            }
        } else {
            vec![PermissionsChainLinkRef {
                allow: self.allow.as_ref().unwrap_or(&PERMISSIONS_NONE_VEC),
                deny: self.deny.as_ref().unwrap_or(&PERMISSIONS_NONE_VEC),
            }]
        }
    }
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

    Files(PathPermission),

    Fonts(StringPermission),

    Env(StringPermission),

    RegistryComponents(RegistryComponentPermission),
    HttpComponents(UrlPermission),
    LocalComponents(LocalComponentPermission),
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
    pub callouts: Option<Callouts>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rigging: Option<Rigging>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub constants: Option<serde_json::Value>,
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
            serde_json::from_str::<Permission>(r#"{"permission":"files"}"#).unwrap(),
            Permission::Files(PathPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"files", "exact": "./foo.txt"}"#)
                .unwrap(),
            Permission::Files(PathPermission::Exact {
                exact: PathBuf::from("./foo.txt")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"files", "within": "./foo"}"#)
                .unwrap(),
            Permission::Files(PathPermission::Within {
                within: PathBuf::from("./foo")
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_font_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"fonts"}"#).unwrap(),
            Permission::Fonts(StringPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"fonts", "exact": "foo"}"#)
                .unwrap(),
            Permission::Fonts(StringPermission::Exact {
                exact: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"fonts", "prefix": "foo"}"#)
                .unwrap(),
            Permission::Fonts(StringPermission::Prefix {
                prefix: String::from("foo")
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"fonts", "suffix": "foo"}"#)
                .unwrap(),
            Permission::Fonts(StringPermission::Suffix {
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
            serde_json::from_str::<Permission>(r#"{"permission":"registry_components"}"#).unwrap(),
            Permission::RegistryComponents(RegistryComponentPermission {
                publisher: None,
                name: None,
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_components", "publisher": "foo"}"#
            )
            .unwrap(),
            Permission::RegistryComponents(RegistryComponentPermission {
                publisher: Some("foo".into()),
                name: None,
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_components", "name": "foo"}"#
            )
            .unwrap(),
            Permission::RegistryComponents(RegistryComponentPermission {
                publisher: None,
                name: Some("foo".into()),
                version: None
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_components", "version": "1.0.0"}"#
            )
            .unwrap(),
            Permission::RegistryComponents(RegistryComponentPermission {
                publisher: None,
                name: None,
                version: Some(VersionReq::parse("1.0.0").unwrap())
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"registry_components", "publisher": "foo", "name": "bar", "version": "1.2.3"}"#
            )
            .unwrap(),
            Permission::RegistryComponents(RegistryComponentPermission {
                publisher: Some("foo".into()),
                name: Some("bar".into()),
                version: Some(VersionReq::parse("1.2.3").unwrap())
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_http_component_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"http_components"}"#).unwrap(),
            Permission::HttpComponents(UrlPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http_components", "exact": "https://example.com/foo.txt"}"#
            )
            .unwrap(),
            Permission::HttpComponents(UrlPermission::Exact {
                exact: Url::parse("https://example.com/foo.txt").unwrap()
            })
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"http_components", "prefix": "https://example.com/foo/"}"#
            )
            .unwrap(),
            Permission::HttpComponents(UrlPermission::Prefix {
                prefix: Url::parse("https://example.com/foo/").unwrap()
            })
        );
    }

    #[slipway_test]
    fn test_deserialize_local_component_permission() {
        assert_eq!(
            serde_json::from_str::<Permission>(r#"{"permission":"local_components"}"#).unwrap(),
            Permission::LocalComponents(LocalComponentPermission::Any {})
        );

        assert_eq!(
            serde_json::from_str::<Permission>(
                r#"{"permission":"local_components", "exact": "foo"}"#
            )
            .unwrap(),
            Permission::LocalComponents(LocalComponentPermission::Exact {
                exact: String::from("foo")
            })
        );
    }
}
