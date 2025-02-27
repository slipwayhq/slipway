use crate::{
    errors::RigError,
    parse::{
        types::parse_component_version,
        url::{process_url_str, ProcessedUrl},
    },
};
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::LazyLock;
use std::{fmt::Display, path::PathBuf, str::FromStr};
use url::Url;

use super::{REGISTRY_PUBLISHER_SEPARATOR, VERSION_SEPARATOR};

pub(crate) static REGISTRY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<publisher>\w+)\.(?<name>\w+)\.(?<version>.+)$").unwrap());

static PASSTHROUGH_STRING: &str = "passthrough";

static SINK_STRING: &str = "sink";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlipwayReference {
    // publisher.name.version
    Registry {
        publisher: String,
        name: String,
        version: Version,
    },

    // file:///absolute-path
    // file:relative-path
    Local {
        path: PathBuf,
    },

    // https://url
    Http {
        url: Url,
    },
    Special(SpecialComponentReference),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpecialComponentReference {
    Pass,
    Sink,
}

impl FromStr for SlipwayReference {
    type Err = RigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == PASSTHROUGH_STRING {
            return Ok(SlipwayReference::Special(SpecialComponentReference::Pass));
        }

        if s == SINK_STRING {
            return Ok(SlipwayReference::Special(SpecialComponentReference::Sink));
        }

        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_component_version(&caps["version"])?;

            return Ok(SlipwayReference::Registry {
                publisher: caps["publisher"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        if let Ok(processed_url) = process_url_str(s) {
            return match processed_url {
                ProcessedUrl::RelativePath(path) => Ok(SlipwayReference::Local { path }),
                ProcessedUrl::AbsolutePath(path) => Ok(SlipwayReference::Local { path }),
                ProcessedUrl::Http(url) => Ok(SlipwayReference::Http { url }),
            };
        }

        Err(RigError::InvalidSlipwayPrimitive {
            primitive_type: stringify!(SlipwayReference).to_string(),
            message: format!("component reference '{}' was not in a valid format", s),
        })
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
            SlipwayReference::Http { url } => f.write_fmt(format_args!("{}", url)),
            SlipwayReference::Special(inner) => f.write_fmt(format_args!("{}", inner)),
        }
    }
}

impl Display for SpecialComponentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialComponentReference::Pass => f.write_str(PASSTHROUGH_STRING),
            SpecialComponentReference::Sink => f.write_str(SINK_STRING),
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
    use common_test_utils::quote;

    mod special_reference_tests {
        use super::*;

        #[test]
        fn it_should_serialize_and_deserialize_pass() {
            let s = r"passthrough";
            let json = quote(s);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let SlipwayReference::Special(SpecialComponentReference::Pass) = reference else {
                panic!("Unexpected reference: {reference}");
            };

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_serialize_and_deserialize_sink() {
            let s = r"sink";
            let json = quote(s);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let SlipwayReference::Special(SpecialComponentReference::Sink) = reference else {
                panic!("Unexpected reference: {reference}");
            };

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }
    }
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
                panic!("Unexpected reference: {reference}");
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
            let url = r"file:///usr/local/rigging.json";
            let json = quote(url);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_local_files() {
            let url = r"file:///usr/local/rigging.json";

            let reference = SlipwayReference::from_str(url).unwrap();

            let SlipwayReference::Local { path } = reference else {
                panic!("Unexpected reference: {reference}");
            };

            assert_eq!(path, PathBuf::from_str("/usr/local/rigging.json").unwrap());
        }

        #[test]
        fn it_should_serialize_and_deserialize_relative_local_files() {
            let url = r"file:../rigging.json";
            let json = quote(url);

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
            let url = r"file:rigging.json";
            let json = quote(url);

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
            let url = r"https://asdf.com/asdf.tar.gz";
            let json = quote(url);

            let reference: SlipwayReference = serde_json::from_str(&json).unwrap();

            let json_out = serde_json::to_string(&reference).unwrap();
            assert_eq!(json, json_out);
        }

        #[test]
        fn it_should_parse_urls() {
            let url_str = r"https://asdf.com/asdf.tar.gz";

            let reference = SlipwayReference::from_str(url_str).unwrap();

            let SlipwayReference::Http { url } = reference else {
                panic!("Unexpected reference: {reference}");
            };

            assert_eq!(url, Url::parse(url_str).unwrap());
        }
    }
}
