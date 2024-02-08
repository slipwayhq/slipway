use std::{collections::HashSet, str::FromStr};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{errors::SlipwayError, parse::types::primitives::ComponentHandle};

use super::find_json_path_strings::FoundJsonPathString;

static COMPONENT_DEPENDENCY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\$\.rigging\.(?<component_handle>\w+)\.output([\.\[]|$)").unwrap());

pub(crate) trait ExtractDependencies {
    fn extract_dependencies(&self) -> Result<HashSet<ComponentHandle>, SlipwayError>;
}

impl ExtractDependencies for Vec<FoundJsonPathString<'_>> {
    fn extract_dependencies(&self) -> Result<HashSet<ComponentHandle>, SlipwayError> {
        let mut result = HashSet::new();

        for found in self {
            if let Some(captures) = COMPONENT_DEPENDENCY_REGEX.captures(&found.path) {
                let component_handle = &captures["component_handle"];
                let component_handle = ComponentHandle::from_str(component_handle)?;
                result.insert(component_handle);
            }
        }

        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn test_extract_dependencies() {
        let json_path_strings = vec![
            FoundJsonPathString {
                path_to: "$.rigging.component1.input".to_string(),
                path: Cow::Borrowed("$.constants.a"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component2.input.a".to_string(),
                path: Cow::Borrowed("$.rigging.component1.output.a"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component2.input.b".to_string(),
                path: Cow::Borrowed("$.rigging.component1.output.b"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component3.input".to_string(),
                path: Cow::Borrowed("$.rigging.component4.output[53].c"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component4.input".to_string(),
                path: Cow::Borrowed("$.rigging.component0.output_not"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component5.input".to_string(),
                path: Cow::Borrowed("$.rigging_component0.output"),
            },
            FoundJsonPathString {
                path_to: "$.rigging.component6.input".to_string(),
                path: Cow::Borrowed("$.rigging.component5.output"),
            },
        ];

        let dependencies = json_path_strings.extract_dependencies().unwrap();

        assert_eq!(
            dependencies,
            vec![
                ComponentHandle::from_str("component1").unwrap(),
                ComponentHandle::from_str("component4").unwrap(),
                ComponentHandle::from_str("component5").unwrap(),
            ]
            .into_iter()
            .collect()
        );
    }
}
