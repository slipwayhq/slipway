use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use super::simple_json_path::SimpleJsonPath;

const JSON_QUERY_PREFIX: &str = "$.";

const REQUIRED_VALUE_CHAR: &str = ".";
const REQUIRED_VALUE_PREFIX: &str = "$.";

const OPTIONAL_VALUE_CHAR: &str = "?";
const OPTIONAL_VALUE_PREFIX: &str = "$?";

const ARRAY_VALUE_CHAR: &str = "*";
const ARRAY_VALUE_PREFIX: &str = "$*";

static COMPONENT_OUTPUT_SHORTCUT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^\$\$(?<value_specifier>[\{}\{}\{}])(?<component_handle>\w+)(?<rest>.*)$",
        REQUIRED_VALUE_CHAR, OPTIONAL_VALUE_CHAR, ARRAY_VALUE_CHAR,
    ))
    .unwrap()
});

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct FoundJsonPathString<'a> {
    pub path_to: Vec<SimpleJsonPath<'a>>,
    pub path: Cow<'a, str>,
    pub path_type: PathType,
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum PathType {
    Array,
    OptionalValue,
    RequiredValue,
}

pub(crate) fn find_json_path_strings(value: &Value) -> Vec<FoundJsonPathString> {
    let mut results = Vec::new();
    let mut current_path = Vec::new();
    find_json_path_strings_inner(value, &mut current_path, &mut results);
    results
}

fn find_json_path_strings_inner<'a>(
    value: &'a Value,
    current_path: &mut Vec<SimpleJsonPath<'a>>,
    results: &mut Vec<FoundJsonPathString<'a>>,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                current_path.push(SimpleJsonPath::Field(key));
                find_json_path_strings_inner(val, current_path, results);
                current_path.pop();
            }
        }
        Value::Array(arr) => {
            for (index, val) in arr.iter().enumerate() {
                current_path.push(SimpleJsonPath::Index(index));
                find_json_path_strings_inner(val, current_path, results);
                current_path.pop();
            }
        }
        Value::String(s) => {
            let maybe_path: Option<(Cow<'_, str>, PathType)> =
                if let Some(captures) = COMPONENT_OUTPUT_SHORTCUT_REGEX.captures(s) {
                    // The string uses the $$ shortcut, so we need to transform it into a proper
                    // JSON path.
                    let component_handle = &captures["component_handle"];
                    let rest = &captures["rest"];

                    let new_path =
                        Cow::Owned("$.rigging.".to_string() + component_handle + ".output" + rest);

                    let value_specifier = &captures["value_specifier"];

                    let path_type = match value_specifier {
                        REQUIRED_VALUE_CHAR => PathType::RequiredValue,
                        OPTIONAL_VALUE_CHAR => PathType::OptionalValue,
                        ARRAY_VALUE_CHAR => PathType::Array,
                        _ => unreachable!(),
                    };

                    Some((new_path, path_type))
                } else if s.starts_with(REQUIRED_VALUE_PREFIX) {
                    // The string is already a valid JSON path.
                    Some((Cow::Borrowed(s), PathType::RequiredValue))
                } else if let Some(rest) = s.strip_prefix(OPTIONAL_VALUE_PREFIX) {
                    // The string uses the $? custom prefix to indicate they want an optional single value result.
                    let new_path = Cow::Owned(JSON_QUERY_PREFIX.to_string() + rest);
                    Some((new_path, PathType::OptionalValue))
                } else if let Some(rest) = s.strip_prefix(ARRAY_VALUE_PREFIX) {
                    // The string uses the $! custom prefix to indicate they require a single value result.
                    let new_path = Cow::Owned(JSON_QUERY_PREFIX.to_string() + rest);
                    Some((new_path, PathType::Array))
                } else {
                    None
                };

            if let Some(path) = maybe_path {
                let result = FoundJsonPathString {
                    path_to: current_path.clone(),
                    path: path.0,
                    path_type: path.1,
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
            "key2": "$$.value2",
            "key3": "$?value1",
            "key4": "$$*value2",
        });
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Field("key1")],
                    path: Cow::Borrowed("$.value1"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Field("key2")],
                    path: Cow::Borrowed("$.rigging.value2.output"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Field("key3")],
                    path: Cow::Borrowed("$.value1"),
                    path_type: PathType::OptionalValue,
                },
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Field("key4")],
                    path: Cow::Borrowed("$.rigging.value2.output"),
                    path_type: PathType::Array,
                },
            ]
        );
    }

    #[test]
    fn test_simple_json_array_with_matching_strings() {
        let value = json!([
            {
                "key1": "$.value1",
                "key2": "$$.value2"
            }
        ]);
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Index(0), SimpleJsonPath::Field("key1")],
                    path: Cow::Borrowed("$.value1"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![SimpleJsonPath::Index(0), SimpleJsonPath::Field("key2")],
                    path: Cow::Borrowed("$.rigging.value2.output"),
                    path_type: PathType::RequiredValue,
                }
            ]
        );
    }

    #[test]
    fn test_nested_json_objects_and_arrays() {
        let value = json!({
            "nested": {
                "array": ["value", "$.value3", 42],
                "key": "$$.nestedValue"
            }
        });
        let results = find_json_path_strings(&value);
        assert_eq!(
            results,
            vec![
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("nested"),
                        SimpleJsonPath::Field("array"),
                        SimpleJsonPath::Index(1)
                    ],
                    path: Cow::Borrowed("$.value3"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("nested"),
                        SimpleJsonPath::Field("key"),
                    ],
                    path: Cow::Borrowed("$.rigging.nestedValue.output"),
                    path_type: PathType::RequiredValue,
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
                path_to: vec![SimpleJsonPath::Field("specialString")],
                path: Cow::Borrowed("$.value4"),
                path_type: PathType::RequiredValue,
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
                    "key4": "$$.level2Value",
                    "deeplyNested": {
                        "key5": "$.deepValue",
                        "array": [
                            1,
                            2,
                            {
                                "arrayNested": "$$.arrayValue"
                            },
                            4
                        ]
                    }
                }
            },
            "level2": {
                "array": ["normal", "$?arrayValue1", 123, "$$*arrayValue2"],
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
                    path_to: vec![SimpleJsonPath::Field("key7")],
                    path: Cow::Borrowed("$.simpleValue"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level1"),
                        SimpleJsonPath::Field("key2")
                    ],
                    path: Cow::Borrowed("$.level1Value"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level1"),
                        SimpleJsonPath::Field("nested"),
                        SimpleJsonPath::Field("deeplyNested"),
                        SimpleJsonPath::Field("array"),
                        SimpleJsonPath::Index(2),
                        SimpleJsonPath::Field("arrayNested")
                    ],
                    path: Cow::Borrowed("$.rigging.arrayValue.output"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level1"),
                        SimpleJsonPath::Field("nested"),
                        SimpleJsonPath::Field("deeplyNested"),
                        SimpleJsonPath::Field("key5")
                    ],
                    path: Cow::Borrowed("$.deepValue"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level1"),
                        SimpleJsonPath::Field("nested"),
                        SimpleJsonPath::Field("key4")
                    ],
                    path: Cow::Borrowed("$.rigging.level2Value.output"),
                    path_type: PathType::RequiredValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level2"),
                        SimpleJsonPath::Field("array"),
                        SimpleJsonPath::Index(1)
                    ],
                    path: Cow::Borrowed("$.arrayValue1"),
                    path_type: PathType::OptionalValue,
                },
                FoundJsonPathString {
                    path_to: vec![
                        SimpleJsonPath::Field("level2"),
                        SimpleJsonPath::Field("array"),
                        SimpleJsonPath::Index(3)
                    ],
                    path: Cow::Borrowed("$.rigging.arrayValue2.output"),
                    path_type: PathType::Array,
                },
            ]
        );
    }
}
