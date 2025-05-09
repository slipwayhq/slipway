use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_FETCH_COMPONENT_TAR_NAME, SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_TEN_COMPONENT_TAR_NAME,
};
use serde::Deserialize;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

#[common_macros::slipway_test_async]
async fn test_fetch_callout_wasm() {
    test_fetch_callout(SLIPWAY_FETCH_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn test_fetch_callout_js() {
    test_fetch_callout(SLIPWAY_FETCH_JS_COMPONENT_TAR_NAME).await;
}
async fn test_fetch_callout(component: &str) {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "url": format!("component://other?type=increment&value=3"),
                    "method": "GET",
                    "headers": {},
                    "body": "{}",
                    "response_type": "text"
                })),
                "other",
                SlipwayReference::Local {
                    path: SLIPWAY_INCREMENT_TEN_COMPONENT_TAR_NAME.into(),
                },
            ),
        )]
        .into_iter()
        .collect(),
    });

    let component_output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();
    let output = serde_json::from_value::<Output>(component_output.value.clone()).unwrap();

    // Expected {"value":13} because increment_ten component adds 10 instead of 1.
    assert_eq!(output.status_code, 200);
    assert!(output.body_bin.is_none());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output.body_text.unwrap()).unwrap(),
        json!({
            "value": 13
        })
    );
}

#[derive(Deserialize)]
struct Output {
    status_code: u16,
    body_text: Option<String>,
    body_bin: Option<Vec<u8>>,
}
