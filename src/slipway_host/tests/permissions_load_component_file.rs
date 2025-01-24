use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::{
    SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME, SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME,
};
use serde_json::json;
use slipway_engine::{
    utils::ch, ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, RunComponentError,
    RunError, SlipwayReference,
};

mod common;

#[test]
fn permissions_load_component_file() {
    let rig = create_rig(Permissions::empty());

    let maybe_output = get_rig_output(rig, "test", Permissions::allow_all());

    let Err(error) = maybe_output else {
        panic!("Expected error, got {:?}", maybe_output);
    };

    match error {
        RunError::RunComponentFailed {
            component_handle,
            component_runner: _,
            error: RunComponentError::RunCallReturnedError { message, inner: _ },
        } => {
            assert_eq!(component_handle, ch("test"));
            assert!(
                message.contains("Component \"test\" does not have permission to access component")
            );
        }
        _ => panic!("Expected permission error, got {:?}", error),
    }
}

fn create_rig(component_permissions: Permissions) -> Rig {
    Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference_callout_override_permissions(
                SlipwayReference::Local {
                    path: SLIPWAY_COMPONENT_FILE_COMPONENT_TAR_NAME.into(),
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
