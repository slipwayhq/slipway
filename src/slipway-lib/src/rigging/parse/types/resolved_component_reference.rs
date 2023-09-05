use std::str::FromStr;

use semver::Version;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::errors::SlipwayError;

use super::unresolved_component_reference::REGISTRY_REGEX;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedComponentReference {
    owner: String,
    name: String,
    version: Version,
}

impl FromStr for ResolvedComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_version(&caps["version"])?;

            return Ok(ResolvedComponentReference {
                owner: caps["owner"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        Err(SlipwayError::InvalidComponentReference(
            "resolved component reference was not in a valid format".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_deserialize_component_reference_from_string() {
        let json = r#""test-owner.test-name#1.2.3""#;

        let reference: ResolvedComponentReference = serde_json::from_str(json).unwrap();

        assert_eq!(reference.owner, "test-owner");
        assert_eq!(reference.name, "test-name");
        assert_eq!(reference.version, Version::new(1, 2, 3));
    }

    #[test]
    fn it_should_parse_component_reference_from_string() {
        let s = r"test-owner.test-name#1.2.3";

        let reference = ResolvedComponentReference::from_str(s).unwrap();

        assert_eq!(reference.owner, "test-owner");
        assert_eq!(reference.name, "test-name");
        assert_eq!(reference.version, Version::new(1, 2, 3));
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_no_version() {
        let s = "test-owner.test-name";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_empty_version() {
        let s = "test-owner.test-name#";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_no_owner() {
        let s = "test-name#1.2.3";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_component_reference_from_string_if_empty_owner() {
        let s = ".test-name#1.2.3";

        let reference_result = ResolvedComponentReference::from_str(s);

        assert!(reference_result.is_err());
    }
}
