use std::str::FromStr;

use serde_json::json;

use jsonpath_rust::{JsonPath, JsonPathValue};

use crate::{errors::AppError, execute::primitives::JsonMetadata, ComponentHandle, ComponentInput};

use super::{
    find_json_path_strings::{FoundJsonPathString, PathType},
    simple_json_path::JsonPathOperations,
};

pub(super) fn evaluate_input(
    component_handle: &ComponentHandle,
    serialized_app_state: &serde_json::Value,
    input: Option<&serde_json::Value>,
    json_path_strings: &Vec<FoundJsonPathString>,
) -> Result<ComponentInput, AppError> {
    let evaluated_input = match input {
        Some(input) => {
            let mut evaluated_input = input.clone();
            for found in json_path_strings {
                let path = JsonPath::from_str(&found.path).map_err(|e| {
                    AppError::InvalidJsonPathExpression {
                        location: found.path_to.to_json_path_string(),
                        error: e,
                    }
                })?;

                let result = path.find_slice(serialized_app_state);

                let extracted_result = match found.path_type {
                    PathType::Array => serde_json::Value::Array(
                        result
                            .into_iter()
                            .filter_map(map_json_ptr_to_value)
                            .collect(),
                    ),
                    PathType::OptionalValue => result
                        .into_iter()
                        .filter_map(map_json_ptr_to_value)
                        .next()
                        .unwrap_or_default(),
                    PathType::RequiredValue => result
                        .into_iter()
                        .filter_map(map_json_ptr_to_value)
                        .next()
                        .ok_or(AppError::ResolveJsonPathFailed {
                            message: format!(
                                r#"The input path "{}" required "{}" to be a value"#,
                                found.path_to.to_prefixed_path_string(
                                    &(component_handle.to_string() + ".input")
                                ),
                                found.path
                            ),
                            state: serialized_app_state.clone(),
                        })?,
                };

                found
                    .path_to
                    .replace(&mut evaluated_input, extracted_result)?;
            }

            let metadata = JsonMetadata::from_value(&evaluated_input);

            ComponentInput {
                value: evaluated_input,
                metadata,
            }
        }
        None => {
            let input_value = json!({});
            let metadata = JsonMetadata::from_value(&input_value);
            ComponentInput {
                value: input_value,
                metadata,
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
