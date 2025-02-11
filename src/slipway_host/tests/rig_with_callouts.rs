use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_INCREMENT_COMPONENT_TAR_NAME, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

#[common_macros::slipway_test_async]
async fn run_no_callout_wasm() {
    run(0, 1, SLIPWAY_INCREMENT_COMPONENT_TAR_NAME).await;
}

#[common_macros::slipway_test_async]
async fn run_two_callouts_wasm() {
    run(2, 3, SLIPWAY_INCREMENT_COMPONENT_TAR_NAME).await;
}

#[common_macros::slipway_test_async]
async fn run_no_callout_js() {
    run(0, 1, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME).await;
}

#[common_macros::slipway_test_async]
async fn run_two_callouts_js() {
    run(2, 3, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME).await;
}

async fn run(ttl: u32, expected_result: u32, component: &str) {
    // Note the callouts are defined in the component,
    // so we don't need to specify them here.
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: component.into(),
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

    let output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();

    assert_eq!(
        output.value,
        json!({
            "value": expected_result
        })
    );
}
