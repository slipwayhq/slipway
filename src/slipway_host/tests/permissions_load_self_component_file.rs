use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME, SLIPWAY_COMPONENT_FILE_JS_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, SlipwayReference,
};

mod common;

#[common_macros::slipway_test_async]
async fn permissions_load_component_file_wasm() {
    permissions_load_component_file(SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_component_file_js() {
    permissions_load_component_file(SLIPWAY_COMPONENT_FILE_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_component_file(component: &str) {
    let rig = create_rig(Permissions::empty(), component);

    let output = get_rig_output(rig, "test", Permissions::allow_all())
        .await
        .unwrap();

    let text = output.value["text"].as_str().unwrap();

    assert!(!text.is_empty());
}

fn create_rig(component_permissions: Permissions, component: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "handle": "",
                    "path": "slipway_component.json",
                    "file_type": "text"
                })),
                component_permissions,
            ),
        )]
        .into_iter()
        .collect(),
    })
}
