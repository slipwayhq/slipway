use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::{
    SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME, SLIPWAY_COMPONENT_FILE_JS_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, RunComponentError, RunError,
    SlipwayReference, utils::ch,
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

    let maybe_output = get_rig_output(rig, "test", Permissions::allow_all()).await;

    let Err(error) = maybe_output else {
        panic!("Expected error, got {:?}", maybe_output);
    };

    match error {
        RunError::RunComponentFailed {
            component_handle,
            component_runner: _,
            error: RunComponentError::RunCallReturnedError { message, inner },
        } => {
            assert_eq!(component_handle, ch("test"));
            assert_messages_contains(
                "Component \"test\" does not have permission to access component",
                &message,
                &inner,
            );
        }
        _ => panic!("Expected permission error, got {:?}", error),
    }
}

fn create_rig(component_permissions: Permissions, component: &str) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override_permissions(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "handle": "other",
                    "path": "input_schema.json",
                    "file_type": "text"
                })),
                "other",
                SlipwayReference::Local {
                    path: SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME.into(),
                },
                component_permissions,
            ),
        )]
        .into_iter()
        .collect(),
    })
}
