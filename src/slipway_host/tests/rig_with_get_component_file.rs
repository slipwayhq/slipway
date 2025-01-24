use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME, SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME,
};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

const SCHEMA_STR: &str = include_str!(
    "../../../src_components/slipway_increment_json_schema_component/input_schema.json"
);
const SCHEMA_BYTES: &[u8] = SCHEMA_STR.as_bytes();

#[test]
fn get_component_file_text() {
    run("text");
}

#[test]
fn get_component_file_binary() {
    run("binary");
}

fn run(file_type: &str) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME.into(),
                },
                Some(json!({
                    "handle": "other",
                    "path": "input_schema.json",
                    "file_type": file_type
                })),
                "other",
                SlipwayReference::Local {
                    path: SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME.into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let component_output = get_rig_output(rig, "test", Permissions::allow_all()).unwrap();

    let output = serde_json::from_value::<Output>(component_output.value.clone()).unwrap();

    if file_type == "text" {
        assert_eq!(output.text, Some(SCHEMA_STR.to_string()));
        assert!(output.bin.is_none());
    } else {
        assert_eq!(output.bin, Some(SCHEMA_BYTES.to_vec()));
        assert!(output.text.is_none());
    };
}

#[derive(Deserialize)]
struct Output {
    text: Option<String>,
    bin: Option<Vec<u8>>,
}
