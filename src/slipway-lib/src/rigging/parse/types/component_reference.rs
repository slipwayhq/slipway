use crate::errors::SlipwayError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{fmt::Display, str::FromStr};

const COMPONENT_REFERENCE_VERSION_SEPARATOR: char = '#';

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComponentReference {
    pub id: String,
    pub version: String,
}

impl ComponentReference {
    pub(crate) const ROOT_ID: &str = ".root";
    const ROOT_VERSION: &str = "0";

    pub fn root() -> Self {
        ComponentReference::exact(
            ComponentReference::ROOT_ID,
            ComponentReference::ROOT_VERSION,
        )
    }

    pub fn is_root(&self) -> bool {
        // We don't bother testing the version, as any version
        // is still technically root if the ID is root.
        self.id == ComponentReference::ROOT_ID
    }

    pub fn exact(id: &str, version: &str) -> Self {
        ComponentReference {
            id: id.to_string(),
            version: version.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ComponentReferenceInner {
    id: String,
    version: String,
}

impl From<ComponentReferenceInner> for ComponentReference {
    fn from(val: ComponentReferenceInner) -> Self {
        ComponentReference {
            id: val.id,
            version: val.version,
        }
    }
}

impl Display for ComponentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}",
            self.id, COMPONENT_REFERENCE_VERSION_SEPARATOR, self.version
        ))
    }
}

impl FromStr for ComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, version) = match s.find(COMPONENT_REFERENCE_VERSION_SEPARATOR) {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => {
                return Err(SlipwayError::InvalidComponentReference(
                    "component reference must be in the form of <id>:<version>".to_string(),
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

            None => match ComponentReferenceInner::deserialize(value) {
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
        let json = r#""test#1""#;

        let reference: ComponentReference = serde_json::from_str(json).unwrap();

        assert_eq!(reference.id, "test");
        assert_eq!(reference.version, "1");
    }

    #[test]
    fn it_should_fail_to_deserialize_component_reference_from_string_if_no_version() {
        let json = "test";

        let reference_result = serde_json::from_str::<ComponentReference>(json);

        assert!(reference_result.is_err());
    }

    #[test]
    fn it_should_deserialize_component_reference_from_struct() {
        let json = r#"{"id": "test", "version": "1"}"#;

        let reference: ComponentReference = serde_json::from_str(json).unwrap();

        assert_eq!(reference.id, "test");
        assert_eq!(reference.version, "1");
    }
}
