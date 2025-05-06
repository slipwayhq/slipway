use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_CONTEXT_COMPONENT_TAR_NAME, SLIPWAY_CONTEXT_JS_COMPONENT_TAR_NAME,
};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference, TEST_TIMEZONE,
};

mod common;

#[common_macros::slipway_test_async]
async fn context_wasm() {
    run(SLIPWAY_CONTEXT_COMPONENT_TAR_NAME).await;
}

#[common_macros::slipway_test_async]
async fn context_js() {
    run(SLIPWAY_CONTEXT_JS_COMPONENT_TAR_NAME).await;
}

async fn run(component: &str) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "context": "$.context"
                })),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let component_output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();

    let output = serde_json::from_value::<Output>(component_output.value.clone()).unwrap();

    assert_eq!(output.tz.as_deref(), Some(TEST_TIMEZONE));

    assert_eq!(
        output.input,
        json!({
            "context": {
                "timezone": TEST_TIMEZONE,
                "device": {
                    "width": 800,
                    "height": 480,
                }
            }
        })
    );
}

#[derive(Deserialize)]
struct Output {
    tz: Option<String>,
    input: serde_json::Value,
}
