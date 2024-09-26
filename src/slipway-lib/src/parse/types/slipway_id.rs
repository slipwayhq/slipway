use std::{fmt::Display, str::FromStr};

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};

use crate::errors::RigError;

use super::{
    parse_component_version,
    primitives::{Name, Publisher},
    slipway_reference::REGISTRY_REGEX,
    REGISTRY_PUBLISHER_SEPARATOR, VERSION_SEPARATOR,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlipwayId {
    pub publisher: Publisher,
    pub name: Name,
    pub version: Version,
}

impl SlipwayId {
    pub fn new(publisher: &Publisher, name: &Name, version: &Version) -> Self {
        SlipwayId {
            publisher: publisher.clone(),
            name: name.clone(),
            version: version.clone(),
        }
    }
}

impl FromStr for SlipwayId {
    type Err = RigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(captures) = REGISTRY_REGEX.captures(s) {
            let version = parse_component_version(&captures["version"])?;

            return Ok(SlipwayId {
                publisher: Publisher::from_str(&captures["publisher"])?,
                name: Name::from_str(&captures["name"])?,
                version,
            });
        }

        Err(RigError::InvalidSlipwayPrimitive {
            primitive_type: stringify!(SlipwayId).to_string(),
            message: format!("id '{}' was not in a valid format", s),
        })
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
        let s = String::deserialize(deserializer)?;
        SlipwayId::from_str(&s).map_err(serde::de::Error::custom)
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
        let s = r"test_publisher.test_name.1.2.3";
        let json = quote(s);

        let id: SlipwayId = serde_json::from_str(&json).unwrap();

        let json_out = serde_json::to_string(&id).unwrap();
        assert_eq!(json, json_out);
    }

    #[test]
    fn it_should_parse_slipway_id_from_string() {
        let s = r"test_publisher.test_name.1.2.3";

        let id = SlipwayId::from_str(s).unwrap();

        assert_eq!(id.publisher.0, "test_publisher");
        assert_eq!(id.name.0, "test_name");
        assert_eq!(id.version, Version::new(1, 2, 3));
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_no_version() {
        let s = "test_publisher.test_name";

        let id = SlipwayId::from_str(s);

        assert!(id.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_empty_version() {
        let s = "test_publisher.test_name.";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_no_publisher() {
        let s = "test_name.1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_from_string_if_empty_publisher() {
        let s = ".test_name.1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_using_hyphens_in_publisher() {
        let s = r"test-publisher.test_name.1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }

    #[test]
    fn it_should_fail_to_parse_slipway_id_using_hyphens_in_name() {
        let s = r"test_publisher.test-name.1.2.3";

        let id_result = SlipwayId::from_str(s);

        assert!(id_result.is_err());
    }
}
