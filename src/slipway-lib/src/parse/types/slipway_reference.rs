use crate::{errors::RigError, parse::types::parse_component_version};
use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use url::Url;

use super::{REGISTRY_PUBLISHER_SEPARATOR, VERSION_SEPARATOR};

pub(crate) static REGISTRY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<publisher>\w+)\.(?<name>\w+)\.(?<version>.+)$").unwrap());

static RELATIVE_FILE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^file:(?<path>[^/].*)$").unwrap());

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlipwayReference {
    // publisher.name#version
    Registry {
        publisher: String,
        name: String,
        version: Version,
    },

    // file://absolute-path
    // file:./relative-path
    Local {
        path: PathBuf,
    },

    // https://url
    Url {
        url: Url,
    },
}

impl FromStr for SlipwayReference {
    type Err = RigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_component_version(&caps["version"])?;

            return Ok(SlipwayReference::Registry {
                publisher: caps["publisher"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        if let Some(caps) = RELATIVE_FILE_REGEX.captures(s) {
            let path = PathBuf::from_str(&caps["path"]).expect("PathBuf::from_str is infallible");
            return Ok(SlipwayReference::Local { path });
        }

        if let Ok(uri) = Url::parse(s) {
            return match uri.scheme() {
                "file" => {
                    let file_path =
                        uri.to_file_path()
                            .map_err(|_| RigError::InvalidSlipwayPrimitive {
                                primitive_type: stringify!(SlipwayReference).to_string(),
                                message: format!("unable to convert file URI to local path: {uri}"),
                            })?;

                    Ok(SlipwayReference::Local { path: file_path })
                }
                "https" => Ok(SlipwayReference::Url { url: uri }),
                other => Err(RigError::InvalidSlipwayPrimitive {
                    primitive_type: stringify!(SlipwayReference).to_string(),
                    message: format!("unsupported URI scheme: {other}"),
                }),
            };
        }

        Err(RigError::InvalidSlipwayPrimitive {
            primitive_type: stringify!(SlipwayReference).to_string(),
            message: format!("component reference '{}' was not in a valid format", s),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GitHubVersion {
    Commitish(String),
    Version(Version),
}

impl FromStr for GitHubVersion {
    type Err = RigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const SEMVER_PREFIX: &str = "semver:";
        if let Some(semver) = s.strip_prefix(SEMVER_PREFIX) {
            let version = parse_component_version(semver)?;
            return Ok(GitHubVersion::Version(version));
        }

        Ok(GitHubVersion::Commitish(s.to_string()))
    }
}

impl Display for GitHubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubVersion::Commitish(commit) => f.write_str(commit),
            GitHubVersion::Version(version) => {
                f.write_fmt(format_args!("{}{}", "semver:", version))
            }
        }
    }
}

impl Display for SlipwayReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlipwayReference::Registry {
                publisher,
                name,
                version,
            } => f.write_fmt(format_args!(
                "{}{}{}{}{}",
                publisher, REGISTRY_PUBLISHER_SEPARATOR, name, VERSION_SEPARATOR, version
            )),
            SlipwayReference::Local { path } => {
                if path.is_relative() {
                    f.write_fmt(format_args!("file:{}", path.display()))
                } else {
                    let url = Url::from_file_path(path).map_err(|_| std::fmt::Error {})?;
                    f.write_fmt(format_args!("{}", url))
                }
            }
            SlipwayReference::Url { url } => f.write_fmt(format_args!("{}", url)),
        }
    }
}

impl<'de> Deserialize<'de> for SlipwayReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SlipwayReference::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for SlipwayReference {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::quote;

    mod registry_tests {
        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_registry() {
            let s = r"test_publisher.test_name.1.2.3";
            let json = quote(s);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_registry_from_string() {
            let s = r"test_publisher.test_name.1.2.3";

            let reference = SlipwayReference::from_str(s).unwrap();

            let SlipwayReference::Registry {
                publisher,
                name,
                version,
            } = reference
            else {
                panic!("Unexpected reference: {reference:?}");
            };

            assert_eq!(publisher, "test_publisher");
            assert_eq!(name, "test_name");
            assert_eq!(version, Version::parse("1.2.3").unwrap());
        }

        #[test]
        fn it_should_fail_to_parse_registry_from_string_if_no_version() {
            let s = "test_publisher.test_name";

            let reference_result = SlipwayReference::from_str(s);

            assert!(reference_result.is_err());
        }

        #[test]
        fn it_should_fail_to_parse_registry_from_string_if_empty_version() {
            let s = "test_publisher.test_name.";

            let reference_result = SlipwayReference::from_str(s);

            assert!(reference_result.is_err());
        }

        #[test]
        fn it_should_fail_to_parse_registry_from_string_if_no_publisher() {
            let s = "test_name.1.2.3";

            let reference_result = SlipwayReference::from_str(s);

            assert!(reference_result.is_err());
        }

        #[test]
        fn it_should_fail_to_parse_registry_from_string_if_empty_publisher() {
            let s = ".test_name.1.2.3";

            let reference_result = SlipwayReference::from_str(s);

            assert!(reference_result.is_err());
        }
    }

    mod local_tests {
        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_local_files() {
            let uri = r"file:///usr/local/rigging.json";
            let json = quote(uri);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_local_files() {
            let uri = r"file:///usr/local/rigging.json";

            let reference = SlipwayReference::from_str(uri).unwrap();

            let SlipwayReference::Local { path } = reference else {
                panic!("Unexpected reference: {reference:?}");
            };

            assert_eq!(path, PathBuf::from_str("/usr/local/rigging.json").unwrap());
        }

        #[test]
        fn it_should_serialize_and_deserialize_relative_local_files() {
            let uri = r"file:../rigging.json";
            let json = quote(uri);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            assert_eq!(
                reference,
                SlipwayReference::Local {
                    path: PathBuf::from_str("../rigging.json").unwrap()
                }
            );

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_serialize_and_deserialize_local_file_name() {
            let uri = r"file:rigging.json";
            let json = quote(uri);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            assert_eq!(
                reference,
                SlipwayReference::Local {
                    path: PathBuf::from_str("rigging.json").unwrap()
                }
            );

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }
    }

    mod url_tests {
        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_urls() {
            let uri = r"https://asdf.com/asdf.tar.gz";
            let json = quote(uri);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_urls() {
            let uri = r"https://asdf.com/asdf.tar.gz";

            let reference = SlipwayReference::from_str(uri).unwrap();

            let SlipwayReference::Url { url } = reference else {
                panic!("Unexpected reference: {reference:?}");
            };

            assert_eq!(url, Url::parse(uri).unwrap());
        }
    }
}
