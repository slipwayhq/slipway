use std::str::FromStr;

use common::get_rig_output;
use serde_json::json;
use slipway_engine::{ComponentHandle, ComponentRigging, Rig, Rigging, SlipwayReference};

mod common;

const SCHEMA_STR: &str =
    include_str!("../../../src_components/slipway_test_json_schema_component/input_schema.json");
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
                    path: "slipway.test.0.0.1.tar".into(),
                },
                Some(json!({
                    "type": "component_file",
                    "handle": "other",
                    "path": "input_schema.json",
                    "file_type": file_type
                })),
                "other",
                SlipwayReference::Local {
                    path: "slipway.test_json_schema.0.0.1.tar".into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "test");

    let expected_length = if file_type == "text" {
        SCHEMA_STR.len()
    } else {
        SCHEMA_BYTES.len()
    };

    assert_eq!(
        output.value,
        json!({
            "value": expected_length
        })
    );
}
