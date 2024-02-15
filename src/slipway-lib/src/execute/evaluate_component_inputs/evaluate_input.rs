use std::str::FromStr;

use serde_json::json;

use jsonpath_rust::{JsonPathInst, JsonPtr};

use crate::{
    errors::SlipwayError,
    execute::{primitives::Hash, ComponentInput},
};

use super::{
    find_json_path_strings::{FoundJsonPathString, PathType},
    simple_json_path::JsonPathOperations,
};

const JSON_PATH_SOURCE_EXTERNAL_PREFIX: &str = "at location \"";
const JSON_PATH_SOURCE_EXTERNAL_SUFFIX: &str = "\"";

pub(super) fn evaluate_input(
    serialized_app_state: &serde_json::Value,
    input: Option<&serde_json::Value>,
    json_path_strings: &Vec<FoundJsonPathString>,
) -> Result<ComponentInput, SlipwayError> {
    let evaluated_input = match input {
        Some(input) => {
            let mut evaluated_input = input.clone();
            for found in json_path_strings {
                let path = JsonPathInst::from_str(&found.path).map_err(|e| {
                    SlipwayError::InvalidJsonPathExpression(
                        format!(
                            "{JSON_PATH_SOURCE_EXTERNAL_PREFIX}{0}{JSON_PATH_SOURCE_EXTERNAL_SUFFIX}",
                            found.path_to.to_json_path_string()
                        ),
                        e,
                    )
                })?;

                let result = path.find_slice(serialized_app_state);

                let extracted_result = match found.path_type {
                    PathType::Array => serde_json::Value::Array(
                        result
                            .into_iter()
                            .map(|v| match v {
                                JsonPtr::NewValue(v) => v,
                                JsonPtr::Slice(s) => s.clone(),
                            })
                            .collect(),
                    ),
                    PathType::Value => result
                        .into_iter()
                        .next()
                        .map(|v| match v {
                            JsonPtr::NewValue(v) => v,
                            JsonPtr::Slice(s) => s.clone(),
                        })
                        .unwrap_or_default(),
                };

                found
                    .path_to
                    .replace(&mut evaluated_input, extracted_result)?;
            }

            let hash = Hash::from_value(&evaluated_input);

            ComponentInput {
                value: evaluated_input,
                hash,
            }
        }
        None => {
            let input_value = json!({});
            let hash = Hash::from_value(&input_value);
            ComponentInput {
                value: input_value,
                hash,
            }
        }
    };

    Ok(evaluated_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_have_tests() {
        todo!();
    }
}
