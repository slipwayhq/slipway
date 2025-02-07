use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_INCREMENT_COMPONENT_TAR_NAME, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_TEN_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

#[test_log::test]
fn run_no_callout_wasm() {
    run(0, 1, SLIPWAY_INCREMENT_COMPONENT_TAR_NAME);
}

#[test_log::test]
fn run_two_callouts_wasm() {
    run(2, 12, SLIPWAY_INCREMENT_COMPONENT_TAR_NAME);
}

#[test_log::test]
fn run_no_callout_js() {
    run(0, 1, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME);
}

#[test_log::test]
fn run_two_callouts_js() {
    run(2, 12, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME);
}

fn run(ttl: u32, expected_result: u32, component: &str) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": ttl,
                    "result_type": "increment"
                })),
                "increment",
                SlipwayReference::Local {
                    path: SLIPWAY_INCREMENT_TEN_COMPONENT_TAR_NAME.into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "test", Permissions::allow_all()).unwrap();

    assert_eq!(
        output.value,
        json!({
            "value": expected_result
        })
    );
}
