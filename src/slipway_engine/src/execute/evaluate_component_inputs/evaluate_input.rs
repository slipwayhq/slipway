use std::str::FromStr;

use serde_json::json;

use jsonpath_rust::{JsonPath, JsonPathValue};

use crate::{ComponentHandle, ComponentInput, errors::RigError, execute::primitives::JsonMetadata};

use super::{
    find_json_path_strings::{FoundJsonPathString, PathType},
    simple_json_path::JsonPathOperations,
};

/// This function evaluates the input of a component by replacing JSON Path strings
/// with the values from the serialized rig state.
/// The serialized rig state is the current state of the rig with any evaluated component
/// outputs populated.
/// The input is the input to the current component, as defined by the rig, which still contains
/// the JSON Path strings.
/// The json_path_strings are the JSON Path strings that were found in the input.
/// The function returns the evaluated input, which is the supplied input with the JSON Path strings
/// replaced with the values from the serialized rig state.
pub(super) fn evaluate_input(
    component_handle: &ComponentHandle,
    serialized_rig_state: &serde_json::Value,
    input: Option<&serde_json::Value>,
    json_path_strings: &Vec<FoundJsonPathString>,
) -> Result<ComponentInput, RigError> {
    let evaluated_input = match input {
        Some(input) => {
            let mut evaluated_input = input.clone();

            // Important: We must evaluate the JSON Path strings in reverse,
            // as we potentially remove items from arrays as we go.
            for found in json_path_strings.iter().rev() {
                let path = JsonPath::from_str(&found.path).map_err(|e| {
                    RigError::InvalidJsonPathExpression {
                        location: found.path_to.to_json_path_string(),
                        error: e,
                    }
                })?;

                let result = path.find_slice(serialized_rig_state);

                let extracted_result = match found.path_type {
                    PathType::Array => Some(serde_json::Value::Array(
                        result
                            .into_iter()
                            .filter_map(map_json_ptr_to_value)
                            .collect(),
                    )),
                    PathType::OptionalValue => {
                        result.into_iter().filter_map(map_json_ptr_to_value).next()
                    }
                    PathType::RequiredValue => Some(
                        result
                            .into_iter()
                            .filter_map(map_json_ptr_to_value)
                            .next()
                            .ok_or(RigError::ResolveJsonPathFailed {
                                message: format!(
                                    r#"The input path "{}" required "{}" to be a value"#,
                                    found.path_to.to_prefixed_path_string(
                                        &(component_handle.to_string() + ".input")
                                    ),
                                    found.path
                                ),
                                state: serialized_rig_state.clone(),
                            })?,
                    ),
                };

                found
                    .path_to
                    .replace(&mut evaluated_input, extracted_result)?;
            }

            let json_metadata = JsonMetadata::from_value(&evaluated_input);

            ComponentInput {
                value: evaluated_input,
                json_metadata,
            }
        }
        None => {
            let input_value = json!({});
            let json_metadata = JsonMetadata::from_value(&input_value);
            ComponentInput {
                value: input_value,
                json_metadata,
            }
        }
    };

    Ok(evaluated_input)
}

fn map_json_ptr_to_value(v: JsonPathValue<'_, serde_json::Value>) -> Option<serde_json::Value> {
    match v {
        JsonPathValue::NewValue(v) => Some(v),
        JsonPathValue::Slice(s, _) => Some(s.clone()),
        JsonPathValue::NoValue => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::ch;

    use super::*;

    #[test]
    fn it_should_populate_required_queries() {
        let serialized_rig_state = json!({
            "constants": {
                "foo": "a",
            },
            "rigging": {
                "component_a": {
                    "output": {
                        "a": 1,
                        "c": 2,
                    }
                }
            }
        });

        let input = json!({
            "foo": "$.constants.foo",
            "a": "$$.component_a.a",
            "c": "$$.component_a.c",
            "array": [
                "$.constants.foo",
                "$$.component_a.a",
                "$$.component_a.c",
            ]
        });

        let json_path_strings =
            super::super::find_json_path_strings::find_json_path_strings(&input);

        let execution_input = evaluate_input(
            &ch("test"),
            &serialized_rig_state,
            Some(&input),
            &json_path_strings,
        )
        .unwrap();

        assert_eq!(
            execution_input.value,
            json!({
                "foo": "a",
                "a": 1,
                "c": 2,
                "array": [
                    "a",
                    1,
                    2
                ]
            })
        );
    }

    #[test]
    fn required_queries_which_are_not_found_should_error() {
        let serialized_rig_state = json!({
            "constants": {
                "foo": "a",
            },
            "rigging": {
                "component_a": {
                    "output": {
                        "a": 1,
                        "c": 2,
                    }
                }
            }
        });

        let input = json!({
            "bar": "$.constants.bar",
        });

        let json_path_strings =
            super::super::find_json_path_strings::find_json_path_strings(&input);

        let maybe_execution_input = evaluate_input(
            &ch("test"),
            &serialized_rig_state,
            Some(&input),
            &json_path_strings,
        );

        assert!(maybe_execution_input.is_err());
    }

    #[test]
    fn it_should_populate_array_queries() {
        let serialized_rig_state = json!({
            "constants": {
                "foo": "a",
            },
            "rigging": {
                "component_a": {
                    "output": {
                        "a": 1,
                        "b": [
                            {
                                "c": 2,
                            },
                            {
                                "c": 3,
                            }
                        ],
                    }
                }
            }
        });

        let input = json!({
            "foo": "$*constants.foo",
            "c": "$$*component_a.b[*].c",
            "array": [
                "$*constants.foo",
                "$$*component_a.b[*].c",
            ]
        });

        let json_path_strings =
            super::super::find_json_path_strings::find_json_path_strings(&input);

        let execution_input = evaluate_input(
            &ch("test"),
            &serialized_rig_state,
            Some(&input),
            &json_path_strings,
        )
        .unwrap();

        assert_eq!(
            execution_input.value,
            json!({
                "foo": ["a"],
                "c": [2, 3],
                "array": [
                    ["a"],
                    [2, 3]
                ]
            })
        );
    }

    #[test]
    fn single_queries_with_array_results_should_take_first_value() {
        let serialized_rig_state = json!({
            "constants": {
                "foo": "a",
            },
            "rigging": {
                "component_a": {
                    "output": {
                        "a": 1,
                        "b": [
                            {
                                "c": 2,
                            },
                            {
                                "c": 3,
                            }
                        ],
                    }
                }
            }
        });

        let input = json!({
            "c": "$$.component_a.b[*].c",
        });

        let json_path_strings =
            super::super::find_json_path_strings::find_json_path_strings(&input);

        let execution_input = evaluate_input(
            &ch("test"),
            &serialized_rig_state,
            Some(&input),
            &json_path_strings,
        )
        .unwrap();

        assert_eq!(
            execution_input.value,
            json!({
                "c": 2,
            })
        );
    }

    #[test]
    fn it_should_correctly_remove_elements_with_optional_queries_which_are_not_found() {
        let serialized_rig_state = json!({
            "constants": {
                "foo": "a",
            },
            "rigging": {
                "component_a": {
                    "output": {
                        "a": 1,
                        "c": 2,
                    }
                }
            }
        });

        let input = json!({
            "foo": "$?constants.foo",
            "bar": "$?constants.bar",
            "a": "$$?component_a.a",
            "b": "$$?component_a.b",
            "c": "$$?component_a.c",
            "array": [
                "$?constants.foo",
                "$?constants.bar",
                "$$?component_a.a",
                "$$?component_a.b",
                "$$?component_a.c",
            ]
        });

        let json_path_strings =
            super::super::find_json_path_strings::find_json_path_strings(&input);

        let execution_input = evaluate_input(
            &ch("test"),
            &serialized_rig_state,
            Some(&input),
            &json_path_strings,
        )
        .unwrap();

        assert_eq!(
            execution_input.value,
            json!({
                "foo": "a",
                "a": 1,
                "c": 2,
                "array": [
                    "a",
                    1,
                    2
                ]
            })
        );
    }
}
