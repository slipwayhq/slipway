use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

static COMPONENT_OUTPUT_SHORTCUT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\$\$(?<component_handle>\w+)(?<rest>.*)$").unwrap());

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct FoundJsonPathString<'a> {
    pub path_to: String,
    pub path: Cow<'a, str>,
}

pub(crate) fn find_json_path_strings<'a>(value: &'a Value) -> Vec<FoundJsonPathString<'a>> {
    let mut results = Vec::new();
    let mut current_path = vec![Cow::Borrowed("$")];
    find_json_path_strings_inner(value, &mut current_path, &mut results);
    results
}

fn find_json_path_strings_inner<'a>(
    value: &'a Value,
    current_path: &mut Vec<Cow<'a, str>>,
    results: &mut Vec<FoundJsonPathString<'a>>,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                current_path.push(Cow::Borrowed("."));
                current_path.push(Cow::Borrowed(key));
                find_json_path_strings_inner(val, current_path, results);
                current_path.pop();
                current_path.pop();
            }
        }
        Value::Array(arr) => {
            for (index, val) in arr.iter().enumerate() {
                current_path.push(Cow::Borrowed("["));
                current_path.push(Cow::Owned(index.to_string()));
                current_path.push(Cow::Borrowed("]"));
                find_json_path_strings_inner(val, current_path, results);
                current_path.pop();
                current_path.pop();
                current_path.pop();
            }
        }
        Value::String(s) => {
            let maybe_path: Option<Cow<'_, str>> =
                if let Some(captures) = COMPONENT_OUTPUT_SHORTCUT_REGEX.captures(s) {
                    let component_handle = &captures["component_handle"];
                    let rest = &captures["rest"];
                    Some(Cow::Owned(
                        "$.rigging.".to_string() + component_handle + ".output" + rest,
                    ))
                } else if s.starts_with("$.") {
                    Some(Cow::Borrowed(s))
                } else {
                    None
                };

            if let Some(path) = maybe_path {
                let result = FoundJsonPathString {
                    path_to: current_path.join(""),
                    path,
                };
                results.push(result);
            }
        }
        _ => {} // For other types, do nothing.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_empty_json() {
        let value = json!({});
        let results = find_json_path_strings(&value);
        assert!(results.is_empty());
    }

    #[test]
    fn test_simple_json_with_matching_strings() {
        let value = json!({
            "key1": "$.value1",
            "key2": "$$value2"
        });
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: "$.key1".to_string(),
                    path: Cow::Borrowed("$.value1")
                },
                FoundJsonPathString {
                    path_to: "$.key2".to_string(),
                    path: Cow::Borrowed("$.rigging.value2.output")
                }
            ]
        );
    }

    #[test]
    fn test_simple_json_array_with_matching_strings() {
        let value = json!([
            {
                "key1": "$.value1",
                "key2": "$$value2"
            }
        ]);
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: "$[0].key1".to_string(),
                    path: Cow::Borrowed("$.value1")
                },
                FoundJsonPathString {
                    path_to: "$[0].key2".to_string(),
                    path: Cow::Borrowed("$.rigging.value2.output")
                }
            ]
        );
    }

    #[test]
    fn test_nested_json_objects_and_arrays() {
        let value = json!({
            "nested": {
                "array": ["value", "$.value3", 42],
                "key": "$$nestedValue"
            }
        });
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: "$.nested.array[1]".to_string(),
                    path: Cow::Borrowed("$.value3")
                },
                FoundJsonPathString {
                    path_to: "$.nested.key".to_string(),
                    path: Cow::Borrowed("$.rigging.nestedValue.output")
                }
            ]
        );
    }

    #[test]
    fn test_json_without_matching_strings() {
        let value = json!({
            "key": "value",
            "array": [1, 2, 3]
        });
        let results = find_json_path_strings(&value);
        assert!(results.is_empty());
    }

    #[test]
    fn test_mixed_types_in_json() {
        let value = json!({
            "string": "normal",
            "number": 123,
            "specialString": "$.value4",
            "bool": true
        });
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![FoundJsonPathString {
                path_to: "$.specialString".to_string(),
                path: Cow::Borrowed("$.value4")
            }]
        );
    }

    #[test]
    fn test_large_json_structure() {
        let value = json!({
            "level1": {
                "key1": "value",
                "key2": "$.level1Value",
                "nested": {
                    "key3": "value",
                    "key4": "$$level2Value",
                    "deeplyNested": {
                        "key5": "$.deepValue",
                        "array": [
                            1,
                            2,
                            {
                                "arrayNested": "$$arrayValue"
                            },
                            4
                        ]
                    }
                }
            },
            "level2": {
                "array": ["normal", "$.arrayValue1", 123, "$$arrayValue2"],
                "key6": "value"
            },
            "key7": "$.simpleValue"
        });

        let mut results = find_json_path_strings(&value);

        results.sort_by(|a, b| a.path_to.cmp(&b.path_to));

        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: "$.key7".to_string(),
                    path: Cow::Borrowed("$.simpleValue")
                },
                FoundJsonPathString {
                    path_to: "$.level1.key2".to_string(),
                    path: Cow::Borrowed("$.level1Value")
                },
                FoundJsonPathString {
                    path_to: "$.level1.nested.deeplyNested.array[2].arrayNested".to_string(),
                    path: Cow::Borrowed("$.rigging.arrayValue.output")
                },
                FoundJsonPathString {
                    path_to: "$.level1.nested.deeplyNested.key5".to_string(),
                    path: Cow::Borrowed("$.deepValue")
                },
                FoundJsonPathString {
                    path_to: "$.level1.nested.key4".to_string(),
                    path: Cow::Borrowed("$.rigging.level2Value.output")
                },
                FoundJsonPathString {
                    path_to: "$.level2.array[1]".to_string(),
                    path: Cow::Borrowed("$.arrayValue1")
                },
                FoundJsonPathString {
                    path_to: "$.level2.array[3]".to_string(),
                    path: Cow::Borrowed("$.rigging.arrayValue2.output")
                },
            ]
        );
    }
}
