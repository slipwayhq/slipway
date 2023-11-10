use crate::errors::SlipwayError;

use self::types::Component;

pub mod types;

pub fn parse_component(input: &str) -> Result<Component, SlipwayError> {
    serde_json::from_str(input).map_err(SlipwayError::RiggingParseFailed)
}

#[cfg(test)]
mod tests {
    use crate::errors::INVALID_COMPONENT_REFERENCE;

    use super::*;

    #[test]
    fn it_should_deserialize_complex_component_rigging() {
        let json = r#"
        {
            "name": "test",
            "publisher": "test-publisher",
            "description": "Test component",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "display_name": "Input 1",
                    "description": "The first input",
                    "schema": {
                        "type": "string"
                    },
                    "default_component": {
                        "reference": "test-publisher.test2#1",
                        "input_overrides": [
                            {
                                "id": "input1",
                                "value": "test"
                            }
                        ]
                    }
                },
                {
                    "id": "input2",
                    "display_name": "Input 2",
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
                "schema_reference": "test-publisher.test2#1"
            }
        }"#;

        let _component = parse_component(json).unwrap();
    }

    #[test]
    fn it_should_provide_a_sensible_message_when_component_reference_cannot_be_parsed() {
        let json = r#"
        {
            "name": "test",
            "publisher": "test-publisher",
            "description": "Test component",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "display_name": "Input 1",
                    "description": "The first input",
                    "schema": {
                        "type": "string"
                    },
                    "default_component": {
                        "reference": "test2/1",
                    }
                }
            ],
            "output": {
                "schema_reference": "test2#1"
            }
        }"#;

        match parse_component(json) {
            Ok(_) => panic!("Expected an error"),
            Err(e) => match e {
                SlipwayError::RiggingParseFailed(e) => {
                    assert!(
                        e.to_string().starts_with(INVALID_COMPONENT_REFERENCE),
                        "Expected error to start with {} but it was {}",
                        INVALID_COMPONENT_REFERENCE,
                        e
                    );
                }
                _ => panic!("Expected a InvalidComponentReference error"),
            },
        }
    }
}
