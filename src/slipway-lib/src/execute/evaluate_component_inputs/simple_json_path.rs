use serde_json::Value;

use crate::errors::SlipwayError;

#[derive(Eq, PartialEq, Debug, Clone, PartialOrd, Ord)]
pub(crate) enum SimpleJsonPath<'a> {
    // Field of an object
    Field(&'a str),
    // Index of an array
    Index(usize),
}

pub(crate) trait JsonPathOperations {
    fn to_json_path_string(&self) -> String;

    fn replace(&self, target: &mut Value, new_value: Value) -> Result<(), SlipwayError>;
}

impl<'a> JsonPathOperations for Vec<SimpleJsonPath<'a>> {
    fn to_json_path_string(&self) -> String {
        let mut result = String::new();
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

    fn replace(&self, target: &mut Value, new_value: Value) -> Result<(), SlipwayError> {
        let mut current = target;
        let path_so_far = vec![SimpleJsonPath::Field("$")];
        for path in self {
            match path {
                SimpleJsonPath::Field(field) => {
                    current = current
                        .as_object_mut()
                        .ok_or(SlipwayError::StepFailed(format!(
                            "Expected {} to be an object",
                            path_so_far.to_json_path_string()
                        )))?
                        .get_mut(*field)
                        .ok_or(SlipwayError::StepFailed(format!(
                            "Expected field {} at {} to exist",
                            field,
                            path_so_far.to_json_path_string()
                        )))?;
                }
                SimpleJsonPath::Index(index) => {
                    current = current
                        .as_array_mut()
                        .ok_or(SlipwayError::StepFailed(format!(
                            "Expected {} to be an array",
                            path_so_far.to_json_path_string()
                        )))?
                        .get_mut(*index)
                        .ok_or(SlipwayError::StepFailed(format!(
                            "Expected index {} at {} to exist",
                            index,
                            path_so_far.to_json_path_string()
                        )))?;
                }
            }
        }
        *current = new_value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_have_tests() {
        todo!();
    }
}
