use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::{
    SLIPWAY_INCREMENT_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_JS_COMPONENT_TAR_NAME,
    SLIPWAY_INCREMENT_JS_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    Callout, ComponentHandle, ComponentRigging, Permission, Permissions,
    RegistryComponentPermission, Rig, Rigging, RunComponentError, RunError, SlipwayReference,
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
            &vec![Permission::RegistryComponents(
                RegistryComponentPermission {
                    name: None,
                    publisher: None,
                    version: None,
                },
            )],
        ),
        component,
        1,
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
    let rig = create_rig(Permissions::allow_all(), component, 1);

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

#[common_macros::slipway_test_async]
async fn permissions_load_component_from_callout_wasm() {
    permissions_load_component_from_callout(
        SLIPWAY_INCREMENT_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME,
    )
    .await;
}
#[common_macros::slipway_test_async]
async fn permissions_load_component_from_callout_js() {
    permissions_load_component_from_callout(
        SLIPWAY_INCREMENT_JS_INVALID_CALLOUT_PERMISSIONS_COMPONENT_TAR_NAME,
    )
    .await;
}
async fn permissions_load_component_from_callout(component: &str) {
    // We need a TTL of 2 here to fail. The call graph looks like this with a TTL of 2:
    // test
    //   - Reference: `slipwayhq.increment_invalid_callout_permissions`
    //   - Has permission from the rig to make a callout to `slipwayhq.increment`.
    // increment
    //   - Reference: `slipwayhq.increment`
    //   - Does not have permission from `increment_invalid_callout_permissions` to make a callout.
    // increment
    //   - Reference: `slipwayhq.increment`
    //   - This callout is the one which fails.
    // To verify this we first check that a TTL of 1 will succeed.
    // Then we check a TTL of 2 will fail.
    {
        // This should be successful with a TTL of 1.
        let rig = create_rig(Permissions::allow_all(), component, 1);
        let maybe_output = get_rig_output(rig, "test", Permissions::allow_all()).await;
        assert!(
            maybe_output.is_ok(),
            "Expected success, got {:?}",
            maybe_output
        );
    }
    {
        // This should fail with a TTL of 2.
        let rig = create_rig(Permissions::allow_all(), component, 2);
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
                    "Component \"test -> increment\" does not have permission to access component",
                    &message,
                    &inner,
                );
            }
            _ => panic!("Expected permission error, got {:?}", error),
        }
    }
    {
        // Finally we're going to test overriding the callout permissions from outside of
        // the `increment_invalid_callout_permissions` component.
        let mut rig = create_rig(Permissions::allow_all(), component, 2);
        rig.rigging
            .components
            .get_mut(&ch("test"))
            .unwrap()
            .callouts = Some(
            vec![(
                ch("increment"),
                Callout {
                    component: SlipwayReference::Local {
                        path: SLIPWAY_INCREMENT_COMPONENT_TAR_NAME.into(),
                    },
                    allow: Some(vec![Permission::RegistryComponents(
                        RegistryComponentPermission {
                            publisher: Some("slipwayhq".into()),
                            name: Some("increment".into()),
                            version: None,
                        },
                    )]),
                    deny: None,
                },
            )]
            .into_iter()
            .collect(),
        );
        let maybe_output = get_rig_output(rig, "test", Permissions::allow_all()).await;
        assert!(
            maybe_output.is_ok(),
            "Expected success, got {:?}",
            maybe_output
        );
    }
}

fn create_rig(component_permissions: Permissions, component: &str, ttl: u32) -> Rig {
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
                    "ttl": ttl,
                    "result_type": "increment"
                })),
                component_permissions,
            ),
        )]
        .into_iter()
        .collect(),
    })
}
