use std::{fmt::Display, str::FromStr};

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    errors::SlipwayError,
    rigging::parse::types::unresolved_component_reference::{
        COMPONENT_REFERENCE_REGISTRY_PUBLISHER_SEPARATOR, COMPONENT_REFERENCE_VERSION_SEPARATOR,
    },
};

use super::unresolved_component_reference::REGISTRY_REGEX;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedComponentReference {
    pub publisher: String,
    pub name: String,
    pub version: Version,
}

impl ResolvedComponentReference {
    pub fn new(publisher: &str, name: &str, version: &Version) -> Self {
        ResolvedComponentReference {
            publisher: publisher.to_string(),
            name: name.to_string(),
            version: version.clone(),
        }
    }

    #[cfg(test)]
    pub fn test(name: &str, version: Version) -> Self {
        use super::TEST_PUBLISHER;

        ResolvedComponentReference {
            publisher: TEST_PUBLISHER.to_string(),
            name: name.to_string(),
            version,
        }
    }
}

impl FromStr for ResolvedComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_version(&caps["version"])?;

            return Ok(ResolvedComponentReference {
                publisher: caps["publisher"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        Err(SlipwayError::InvalidComponentReference(
            "resolved component reference was not in a valid format".to_string(),
        ))
    }
}

impl Display for ResolvedComponentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}{}{}",
            self.publisher,
            COMPONENT_REFERENCE_REGISTRY_PUBLISHER_SEPARATOR,
            self.name,
            COMPONENT_REFERENCE_VERSION_SEPARATOR,
            self.version
        ))
    }
}

fn parse_version(version_string: &str) -> Result<Version, SlipwayError> {
    Version::parse(version_string)
        .map_err(|e| SlipwayError::InvalidComponentReference(e.to_string()))
}

impl<'de> Deserialize<'de> for ResolvedComponentReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match value.as_str() {
            Some(reference_as_string) => ResolvedComponentReference::from_str(reference_as_string)
                .map_err(serde::de::Error::custom),

            None => Err(serde::de::Error::custom(
                "reference should be in string format",
            )),
        }
    }
}

impl Serialize for ResolvedComponentReference {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::quote;

    #[test]
    fn it_should_serialize_and_deserialize_component_reference() {
        let s = r"test-publisher.test-name#1.2.3";
        let json = quote(s);

        let reference: ResolvedComponentReference = serde_json::from_str(&json).unwrap();

        let json_out = serde_json::to_string(&reference).unwrap();
        assert_eq!(json, json_out);
    }

    #[test]
    fn it_should_parse_component_reference_from_string() {
        let s = r"test-publisher.test-name#1.2.3";

        let reference = ResolvedComponentReference::from_str(s).unwrap();

        assert_eq!(reference.publisher, "test-publisher");
        assert_eq!(reference.name, "test-name");
        assert_eq!(reference.version, Version::new(1, 2, 3));
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_no_version() {
        let s = "test-publisher.test-name";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_empty_version() {
        let s = "test-publisher.test-name#";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_no_publisher() {
        let s = "test-name#1.2.3";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_empty_publisher() {
        let s = ".test-name#1.2.3";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }
}
