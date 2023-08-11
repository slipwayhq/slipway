use crate::SlipwayError;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
#[serde(from = "ComponentReferenceEnum")]
pub(crate) struct ComponentReference {
    pub id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
struct ComponentReferenceInner {
    id: String,
    version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ComponentReferenceEnum {
    #[serde(deserialize_with = "deserialize_component_reference_from_str")]
    FromStr(ComponentReferenceInner),
    FromDict(ComponentReferenceInner),
}

impl From<ComponentReferenceEnum> for ComponentReference {
    fn from(e: ComponentReferenceEnum) -> ComponentReference {
        let h = match e {
            ComponentReferenceEnum::FromStr(h) => h,
            ComponentReferenceEnum::FromDict(h) => h,
        };
        ComponentReference {
            id: h.id,
            version: h.version,
        }
    }
}

impl FromStr for ComponentReference {
    type Err = SlipwayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, version) = match s.find(':') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => {
                return Err(SlipwayError::InvalidRigging(
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

fn deserialize_component_reference_from_str<'de, D>(
    deserializer: D,
) -> Result<ComponentReferenceInner, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let component_reference = value.parse::<ComponentReference>();
    match component_reference {
        Ok(component_reference) => Ok(ComponentReferenceInner {
            id: component_reference.id,
            version: component_reference.version,
        }),
        Err(e) => Err(serde::de::Error::custom(e.to_string())),
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
