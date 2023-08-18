use crate::errors::SlipwayError;

use self::types::Component;

pub mod types;

pub fn parse_component(input: &str) -> Result<Component, SlipwayError> {
    serde_json::from_str(input).map_err(SlipwayError::RiggingParseFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_deserialize_complex_component_rigging() {
        let json = r#"
        {
            "id": "test",
            "description": "Test component",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input 1",
                    "description": "The first input",
                    "schema": {
                        "type": "string"
                    },
                    "default_component": {
                        "reference": {
                            "id": "test2",
                            "version": "1.0.0"
                        },
                        "inputs": [
                            {
                                "id": "input1",
                                "value": "test"
                            }
                        ]
                    }
                },
                {
                    "id": "input2",
                    "name": "Input 2",
                    "description": "The second input",
                    "schema": {
                        "type": "string"
                    },
                    "default_value": "test"
                }
            ],
            "output": {
                "schema": {
                    "type": "string"
                },
                "schema_reference": {
                    "id": "test2",
                    "version": "1.0.0"
                }
            }
        }"#;

        let _component = parse_component(json).unwrap();
    }
}
