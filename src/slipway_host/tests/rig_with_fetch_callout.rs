use std::str::FromStr;

use common::get_rig_output;
use serde_json::json;
use slipway_engine::{ComponentHandle, ComponentRigging, Rig, Rigging, SlipwayReference};

mod common;

#[test]
fn test_fetch_callout() {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: "slipway.test.0.0.1.tar".into(),
                },
                Some(json!({
                    "type": "http",
                    "url": format!("component://other?type=increment&value=3"),
                    "method": "GET",
                    "headers": {},
                    "expected_status_code": 200,
                    "body": "{}",
                    "response_type": "text"
                })),
                "other",
                SlipwayReference::Local {
                    path: "slipway.test_2.0.0.1.tar".into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let output = get_rig_output(rig, "test");

    // Expected: {"value":4}
    let expected_length = 11;

    assert_eq!(
        output.value,
        json!({
            "value": expected_length
        })
    );
}
