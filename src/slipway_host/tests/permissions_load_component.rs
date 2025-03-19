use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::{
    SLIPWAY_INCREMENT_COMPONENT_TAR_NAME, SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permission, Permissions, RegistryComponentPermission, Rig,
    Rigging, RunComponentError, RunError, SlipwayReference,
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    utils::ch,
};

mod common;

#[common_macros::slipway_test_async]
async fn permissions_load_component_from_rig_wasm() {
    permissions_load_component_from_rig(SLIPWAY_INCREMENT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_component_from_rig_js() {
    permissions_load_component_from_rig(SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_component_from_rig(component: &str) {
    let rig = create_rig(
        Permissions::new(
            &vec![Permission::All],
            &vec![Permission::RegistryComponent(RegistryComponentPermission {
                name: None,
                publisher: None,
                version: None,
            })],
        ),
        component,
    );

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

#[common_macros::slipway_test_async]
async fn permissions_load_component_from_component_wasm() {
    permissions_load_component_from_component(SLIPWAY_INCREMENT_COMPONENT_TAR_NAME).await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_component_from_component_js() {
    permissions_load_component_from_component(SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME).await;
}
async fn permissions_load_component_from_component(component: &str) {
    let rig = create_rig(Permissions::allow_all(), component);

    let maybe_output = get_rig_output(rig, "test", Permissions::empty()).await;

    let Err(error) = maybe_output else {
        panic!("Expected error, got {:?}", maybe_output);
    };

    match error {
        RunError::ComponentLoadFailed(ComponentLoadError {
            reference,
            error: ComponentLoadErrorInner::PermissionDenied { message, inner },
        }) => {
            assert_eq!(
                reference,
                Box::new(SlipwayReference::Local {
                    path: component.into(),
                })
            );
            assert_messages_contains(
                "Rig does not have permission to access component",
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
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: component.into(),
                },
                Some(json!({
                    "type": "callout_increment",
                    "value": 0,
                    "ttl": 1,
                    "result_type": "increment"
                })),
                component_permissions,
            ),
        )]
        .into_iter()
        .collect(),
    })
}
