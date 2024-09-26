use std::sync::Arc;

use crate::errors::{RigError, ComponentLoadErrorInner};

use self::types::{Rig, Component};

mod parse_schema;
pub(crate) mod types;

pub use parse_schema::parse_schema;

pub fn parse_rig(input: &str) -> Result<Rig, RigError> {
    serde_json::from_str(input).map_err(|error| RigError::RigParseFailed { error })
}

pub fn parse_component(
    input: &str,
) -> Result<Component<serde_json::Value>, ComponentLoadErrorInner> {
    serde_json::from_str(input)
        .map_err(|e| ComponentLoadErrorInner::DefinitionParseFailed { error: Arc::new(e) })
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    fn it_should_parse_examples_folder<T, TParse, TError>(examples_dir: &str, parse_method: TParse)
    where
        TParse: Fn(&str) -> Result<T, TError>,
        TError: Debug,
    {
        let mut parsed_files = 0;
        for entry in std::fs::read_dir(examples_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let file_contents = std::fs::read_to_string(path.clone()).unwrap();
                let _rig = parse_method(&file_contents).unwrap_or_else(|error| {
                    panic!("Failed to parse {}: {:?}", path.display(), error)
                });
                parsed_files += 1;
            }
        }
        assert!(parsed_files > 0);
    }

    /// This test loads each JSON file from from the examples/rigs directory
    /// and parses it using `parse_rig`.
    /// There should be no errors. There should be at least one file parsed.
    #[test]
    fn it_should_parse_example_rigs() {
        let examples_root_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");

        it_should_parse_examples_folder(
            examples_root_dir.join("rigs").to_str().unwrap(),
            parse_rig,
        );

        it_should_parse_examples_folder(
            examples_root_dir.join("components").to_str().unwrap(),
            parse_component,
        );
    }

    #[test]
    fn it_should_provide_a_sensible_message_when_component_reference_cannot_be_parsed() {
        let json = r#"
        {
            "publisher": "slipway",
            "name": "weather",
            "version": "0.0.1",
            "rigging": {
                "weather_url_resolver": {
                    "component": "invalid-component-reference"
                }
            }
          }"#;

        let expected = "Invalid SlipwayReference:";
        match parse_rig(json) {
            Ok(_) => panic!("Expected an error"),
            Err(e) => match e {
                RigError::RigParseFailed { error } => {
                    assert!(
                        error.to_string().starts_with(expected),
                        "Expected error to start with \"{}\" but it was \"{}\"",
                        expected,
                        error
                    );
                }
                _ => panic!("Expected a InvalidComponentReference error"),
            },
        }
    }

    #[test]
    fn it_should_provide_a_sensible_message_when_duplicate_rigging_keys() {
        let json = r#"
        {
            "publisher": "slipway",
            "name": "weather",
            "version": "0.0.1",
            "rigging": {
                "weather_url_resolver": {
                    "component": "a.b.1.0.0"
                },
                "weather_url_resolver": {
                    "component": "a.b.2.0.0"
                }
            }
          }"#;

        const DUPLICATE_RIGGING_KEY: &str = "invalid entry: found duplicate key";
        match parse_rig(json) {
            Ok(_) => panic!("Expected an error"),
            Err(e) => match e {
                RigError::RigParseFailed { error } => {
                    assert!(
                        error.to_string().starts_with(DUPLICATE_RIGGING_KEY),
                        "Expected error to start with \"{}\" but it was \"{}\"",
                        DUPLICATE_RIGGING_KEY,
                        error
                    );
                }
                _ => panic!("Expected a duplicate key error"),
            },
        }
    }
}
