use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::SLIPWAY_FRAGMENT_COMPONENT_TAR_NAME;
use serde_json::json;
use slipway_engine::{ComponentHandle, ComponentRigging, Rig, Rigging, SlipwayReference};

mod common;

#[test]
fn run_no_callout() {
    run(0, 2);
}

#[test]
fn run_one_callout() {
    run(1, 3);
}

#[test]
fn run_two_callouts() {
    run(2, 4);
}

fn run(ttl: u32, expected_result: u32) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("frag").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: SLIPWAY_FRAGMENT_COMPONENT_TAR_NAME.into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": ttl,
                    "result_type": "increment"
                })),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "frag");

    assert_eq!(
        output.value,
        json!({
            "value": expected_result
        })
    );
}
