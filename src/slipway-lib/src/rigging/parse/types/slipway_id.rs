use std::{fmt::Display, str::FromStr};

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::errors::SlipwayError;

use super::{
    parse_component_version, slipway_reference::REGISTRY_REGEX, REGISTRY_PUBLISHER_SEPARATOR,
    VERSION_SEPARATOR,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlipwayId {
    pub publisher: String,
    pub name: String,
    pub version: Version,
}

impl SlipwayId {
    pub fn new(publisher: &str, name: &str, version: &Version) -> Self {
        SlipwayId {
            publisher: publisher.to_string(),
            name: name.to_string(),
            version: version.clone(),
        }
    }

    #[cfg(test)]
    pub fn for_test(name: &str, version: Version) -> Self {
        use super::TEST_PUBLISHER;

        SlipwayId {
            publisher: TEST_PUBLISHER.to_string(),
            name: name.to_string(),
            version,
        }
    }
}

impl FromStr for SlipwayId {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = REGISTRY_REGEX.captures(s) {
            let version = parse_component_version(&caps["version"])?;

            return Ok(SlipwayId {
                publisher: caps["publisher"].to_string(),
                name: caps["name"].to_string(),
                version,
            });
        }

        Err(SlipwayError::InvalidSlipwayId(format!(
            "id '{}' was not in a valid format",
            s
        )))
    }
}

impl Display for SlipwayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}{}{}",
            self.publisher,
            REGISTRY_PUBLISHER_SEPARATOR,
            self.name,
            VERSION_SEPARATOR,
            self.version
        ))
    }
}

impl<'de> Deserialize<'de> for SlipwayId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match value.as_str() {
            Some(id_as_string) => {
                SlipwayId::from_str(id_as_string).map_err(serde::de::Error::custom)
            }

            None => Err(serde::de::Error::custom("id should be in string format")),
        }
    }
}

impl Serialize for SlipwayId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::quote;

    #[test]
    fn it_should_serialize_and_deserialize_slipway_id() {
        let s = r"test-publisher.test-name#1.2.3";
        let json = quote(s);

        let id: SlipwayId = serde_json::from_str(&json).unwrap();

        let json_out = serde_json::to_string(&id).unwrap();
        assert_eq!(json, json_out);
    }

    #[test]
    fn it_should_parse_slipway_id_from_string() {
        let s = r"test-publisher.test-name#1.2.3";

        let id = SlipwayId::from_str(s).unwrap();

        assert_eq!(id.publisher, "test-publisher");
        assert_eq!(id.name, "test-name");
        assert_eq!(id.version, Version::new(1, 2, 3));
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_no_version() {
        let s = "test-publisher.test-name";

        let id = SlipwayId::from_str(s);

        assert!(id.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_empty_version() {
        let s = "test-publisher.test-name#";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_no_publisher() {
        let s = "test-name#1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_empty_publisher() {
        let s = ".test-name#1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }
}
