use serde_json::Value;

use crate::errors::RigError;

#[derive(Eq, PartialEq, Debug, Clone, PartialOrd, Ord)]
pub(super) enum SimpleJsonPath<'a> {
    // Field of an object
    Field(&'a str),
    // Index of an array
    Index(usize),
}

pub(super) trait JsonPathOperations {
    fn to_json_path_string(&self) -> String;

    fn to_prefixed_path_string(&self, prefix: &str) -> String;

    fn replace(&self, target: &mut Value, new_value: Option<Value>) -> Result<(), RigError>;
}

impl JsonPathOperations for Vec<SimpleJsonPath<'_>> {
    fn to_json_path_string(&self) -> String {
        self.to_prefixed_path_string("$")
    }

    fn to_prefixed_path_string(&self, prefix: &str) -> String {
        let mut result = prefix.to_string();
        for path in self {
            match path {
                SimpleJsonPath::Field(field) => {
                    result.push_str(&format!(".{}", field));
                }
                SimpleJsonPath::Index(index) => {
                    result.push_str(&format!("[{}]", index));
                }
            }
        }
        result
    }

    fn replace(&self, target: &mut Value, new_value: Option<Value>) -> Result<(), RigError> {
        let mut current = target;
        let path_so_far = vec![SimpleJsonPath::Field("$")];
        for (i, path) in self.iter().enumerate() {
            match path {
                SimpleJsonPath::Field(field) => {
                    let o = current.as_object_mut().ok_or(RigError::StepFailed {
                        error: format!(
                            "Expected {} to be an object",
                            path_so_far.to_json_path_string()
                        ),
                    })?;

                    if i == self.len() - 1 && new_value.is_none() {
                        o.remove(*field);
                        return Ok(());
                    }

                    current = o.get_mut(*field).ok_or(RigError::StepFailed {
                        error: format!(
                            "Expected field {} at {} to exist",
                            field,
                            path_so_far.to_json_path_string()
                        ),
                    })?;
                }
                SimpleJsonPath::Index(index) => {
                    let a = current.as_array_mut().ok_or(RigError::StepFailed {
                        error: format!(
                            "Expected {} to be an array",
                            path_so_far.to_json_path_string()
                        ),
                    })?;

                    if i == self.len() - 1 && new_value.is_none() {
                        // Important: Because we remove items from the array, we must ensure we
                        // evaluate the found json path strings  in reverse order.
                        a.remove(*index);
                        return Ok(());
                    }

                    current = a.get_mut(*index).ok_or(RigError::StepFailed {
                        error: format!(
                            "Expected index {} at {} to exist",
                            index,
                            path_so_far.to_json_path_string()
                        ),
                    })?;
                }
            }
        }
        match new_value {
            Some(new_value) => {
                *current = new_value;
            }
            None => {
                unreachable!("new_value should never be None here, as we handle None in the loop");
            }
        }

        Ok(())
    }
}

// Note: There is more test coverage in the tests for the `execute` module.
#[cfg(test)]
mod tests {
    use super::*;

    mod to_json_path_string {
        use super::{JsonPathOperations, SimpleJsonPath};

        #[test]
        fn it_should_create_json_path_string() {
            let path = vec![
                SimpleJsonPath::Field("a"),
                SimpleJsonPath::Field("b"),
                SimpleJsonPath::Index(0),
                SimpleJsonPath::Field("c"),
                SimpleJsonPath::Field("e"),
            ];

            let result = path.to_json_path_string();

            assert_eq!(result, "$.a.b[0].c.e");
        }
    }

    mod replace {
        use serde_json::json;

        use super::{JsonPathOperations, SimpleJsonPath};

        #[test]
        fn it_should_replace_values_in_json() {
            let target = json!({
                "a": {
                    "b": [
                        {
                            "c": {
                                "d": 1,
                                "e": 2,
                            }
                        }
                    ]
                }
            });

            let mut target_mut = target.clone();

            let new_value = Some(json!({ "f": 3 }));

            let path = vec![
                SimpleJsonPath::Field("a"),
                SimpleJsonPath::Field("b"),
                SimpleJsonPath::Index(0),
                SimpleJsonPath::Field("c"),
                SimpleJsonPath::Field("e"),
            ];

            path.replace(&mut target_mut, new_value).unwrap();

            assert_eq!(
                target_mut,
                json!({
                    "a": {
                        "b": [
                            {
                                "c": {
                                    "d": 1,
                                    "e": {
                                        "f": 3
                                    },
                                }
                            }
                        ]
                    }
                })
            );
        }

        #[test]
        fn it_should_replace_values_in_json_array() {
            let target = json!({
                "a": {
                    "b": [
                        {
                            "c": [1, 2, 3]
                        }
                    ]
                }
            });

            let mut target_mut = target.clone();

            let new_value = Some(json!(4));

            let path = vec![
                SimpleJsonPath::Field("a"),
                SimpleJsonPath::Field("b"),
                SimpleJsonPath::Index(0),
                SimpleJsonPath::Field("c"),
                SimpleJsonPath::Index(1),
            ];

            path.replace(&mut target_mut, new_value).unwrap();

            assert_eq!(
                target_mut,
                json!({
                    "a": {
                        "b": [
                            {
                                "c": [1, 4, 3]
                            }
                        ]
                    }
                })
            );
        }

        #[test]
        fn it_should_remove_values_in_json() {
            let target = json!({
                "a": {
                    "b": [
                        {
                            "c": {
                                "d": 1,
                                "e": 2,
                            }
                        }
                    ]
                }
            });

            let mut target_mut = target.clone();

            let new_value = None;

            let path = vec![
                SimpleJsonPath::Field("a"),
                SimpleJsonPath::Field("b"),
                SimpleJsonPath::Index(0),
                SimpleJsonPath::Field("c"),
                SimpleJsonPath::Field("e"),
            ];

            path.replace(&mut target_mut, new_value).unwrap();

            assert_eq!(
                target_mut,
                json!({
                    "a": {
                        "b": [
                            {
                                "c": {
                                    "d": 1
                                }
                            }
                        ]
                    }
                })
            );
        }

        #[test]
        fn it_should_remove_values_in_json_array() {
            let target = json!({
                "a": {
                    "b": [
                        {
                            "c": [1, 2, 3]
                        }
                    ]
                }
            });

            let mut target_mut = target.clone();

            let new_value = None;

            let path = vec![
                SimpleJsonPath::Field("a"),
                SimpleJsonPath::Field("b"),
                SimpleJsonPath::Index(0),
                SimpleJsonPath::Field("c"),
                SimpleJsonPath::Index(1),
            ];

            path.replace(&mut target_mut, new_value).unwrap();

            assert_eq!(
                target_mut,
                json!({
                    "a": {
                        "b": [
                            {
                                "c": [1, 3]
                            }
                        ]
                    }
                })
            );
        }
    }
}
