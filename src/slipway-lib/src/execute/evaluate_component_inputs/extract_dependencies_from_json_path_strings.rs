use std::{collections::HashSet, str::FromStr};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{errors::SlipwayError, parse::types::primitives::ComponentHandle};

use super::find_json_path_strings::FoundJsonPathString;

/// This regex matches any JSON path string that references either the output
/// or input of a component.
/// We match the input because the references components inputs could contain references
/// that need to be resolved. We could follow the transitive references until we find
/// an output reference, and add a dependency to that component, but that would add
/// complexity to a niche scenario. This solution is simpler and only results in reduced
/// parallelism in these unusual cases.
static COMPONENT_DEPENDENCY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\$\.rigging\.(?<component_handle>\w+)\.(output|input)([\.\[]|$)").unwrap()
});

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

    use crate::execute::evaluate_component_inputs::simple_json_path::SimpleJsonPath;

    use super::super::find_json_path_strings::PathType;

    use super::*;

    #[test]
    fn test_extract_dependencies() {
        let json_path_strings = vec![
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component1"),
                    SimpleJsonPath::Field("input"),
                ],
                path: Cow::Borrowed("$.constants.a"),
                path_type: PathType::Array,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component2"),
                    SimpleJsonPath::Field("input"),
                    SimpleJsonPath::Field("a"),
                ],
                path: Cow::Borrowed("$.rigging.component1.output.a"),
                path_type: PathType::OptionalValue,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component2"),
                    SimpleJsonPath::Field("input"),
                    SimpleJsonPath::Field("b"),
                ],
                path: Cow::Borrowed("$.rigging.component1.output.b"),
                path_type: PathType::Array,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component3"),
                    SimpleJsonPath::Field("input"),
                ],
                path: Cow::Borrowed("$.rigging.component4.output[53].c"),
                path_type: PathType::RequiredValue,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component4"),
                    SimpleJsonPath::Field("input"),
                ],
                path: Cow::Borrowed("$.rigging.component0.output_not"),
                path_type: PathType::Array,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component5"),
                    SimpleJsonPath::Field("input"),
                ],
                path: Cow::Borrowed("$.rigging_component0.output"),
                path_type: PathType::OptionalValue,
            },
            FoundJsonPathString {
                path_to: vec![
                    SimpleJsonPath::Field("rigging"),
                    SimpleJsonPath::Field("component6"),
                    SimpleJsonPath::Field("input"),
                ],
                path: Cow::Borrowed("$.rigging.component5.output"),
                path_type: PathType::Array,
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
