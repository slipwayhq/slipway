use std::str::FromStr;

use common::{assert_messages_contains, get_rig_output};
use common_test_utils::SLIPWAY_INCREMENT_COMPONENT_TAR_NAME;
use serde_json::json;
use slipway_engine::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    utils::ch,
    ComponentHandle, ComponentRigging, LocalComponentPermission, Permission, Permissions, Rig,
    Rigging, RunComponentError, RunError, SlipwayReference,
};

mod common;

#[test]
fn permissions_load_component_from_rig() {
    let rig = create_rig(Permissions::new(
        &vec![Permission::All],
        &vec![Permission::LocalComponent(LocalComponentPermission::Any {})],
    ));

    let maybe_output = get_rig_output(rig, "test", Permissions::allow_all());

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

#[test]
fn permissions_load_component_from_component() {
    let rig = create_rig(Permissions::allow_all());

    let maybe_output = get_rig_output(rig, "test", Permissions::empty());

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
                    path: SLIPWAY_INCREMENT_COMPONENT_TAR_NAME.into(),
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

fn create_rig(component_permissions: Permissions) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_permissions(
                SlipwayReference::Local {
                    path: SLIPWAY_INCREMENT_COMPONENT_TAR_NAME.into(),
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
