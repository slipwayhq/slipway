use std::str::FromStr;

use common::get_rig_output;
use serde_json::json;
use slipway_engine::{ComponentHandle, ComponentRigging, Rig, Rigging, SlipwayReference};

mod common;

#[test_log::test]
fn run_no_callout() {
    run(0, 1);
}

#[test_log::test]
fn run_two_callouts() {
    run(2, 12);
}

fn run(ttl: u32, expected_result: u32) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: "slipway.test.0.0.1.tar".into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": ttl,
                    "result_type": "increment"
                })),
                "test",
                SlipwayReference::Local {
                    path: "slipway.test_2.0.0.1.tar".into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "test");

    assert_eq!(
        output.value,
        json!({
            "value": expected_result
        })
    );
}
