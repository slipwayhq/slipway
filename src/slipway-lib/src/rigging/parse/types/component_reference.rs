use crate::errors::SlipwayError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Serialize)]
// #[serde(from = "ComponentReferenceEnum")]
pub struct ComponentReference {
    pub id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
struct ComponentReferenceFull {
    id: String,
    version: String,
}

impl From<ComponentReferenceFull> for ComponentReference {
    fn from(val: ComponentReferenceFull) -> Self {
        ComponentReference {
            id: val.id,
            version: val.version,
        }
    }
}

impl FromStr for ComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, version) = match s.find(':') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => {
                return Err(SlipwayError::InvalidComponentReference(
                    "Component reference must be in the form of <id>:<version>".to_string(),
                ))
            }
        };

        Ok(ComponentReference {
            id: id.to_string(),
            version: version.to_string(),
        })
    }
}

impl<'de> Deserialize<'de> for ComponentReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match value.as_str() {
            Some(reference_as_string) => {
                ComponentReference::from_str(reference_as_string).map_err(serde::de::Error::custom)
            }

            None => match ComponentReferenceFull::deserialize(value) {
                Ok(cr) => Ok(cr.into()),
                Err(e) => Err(serde::de::Error::custom(e.to_string())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_deserialize_component_reference_from_string() {
        let json = r#""test:1.0.0""#;

        let reference: ComponentReference = serde_json::from_str(json).unwrap();

        assert_eq!(reference.id, "test");
        assert_eq!(reference.version, "1.0.0");
    }

    #[test]
    fn it_should_fail_to_deserialize_component_reference_from_string_if_no_version() {
        let json = "test";

        let reference_result = serde_json::from_str::<ComponentReference>(json);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_deserialize_component_reference_from_struct() {
        let json = r#"{"id": "test", "version": "1.0.0"}"#;

        let reference: ComponentReference = serde_json::from_str(json).unwrap();

        assert_eq!(reference.id, "test");
        assert_eq!(reference.version, "1.0.0");
    }
}
