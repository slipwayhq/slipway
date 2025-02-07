use std::str::FromStr;

use common::get_rig_output;
use common_test_utils::SLIPWAY_FETCH_ERROR_JS_COMPONENT_TAR_NAME;
use serde_json::json;
use slipway_engine::{
    ComponentHandle, ComponentRigging, Permissions, Rig, Rigging, RunComponentError, RunError,
    SlipwayReference,
};

mod common;

#[test_log::test]
fn deserialize_argument_error_propagation() {
    let rig: Rig = Rig::for_test(Rigging {
        components: [(
            ComponentHandle::from_str("test").unwrap(),
            ComponentRigging::for_test_with_reference(
                SlipwayReference::Local {
                    path: SLIPWAY_FETCH_ERROR_JS_COMPONENT_TAR_NAME.into(),
                },
                Some(json!({})),
            ),
        )]
        .into_iter()
        .collect(),
    });

    let maybe_component_output = get_rig_output(rig, "test", Permissions::allow_all());

    let Err(error) = maybe_component_output else {
        panic!("Expected component to error");
    };

    let RunError::RunComponentFailed {
        component_handle,
        component_runner: _,
        error,
    } = error
    else {
        panic!("Expected RunComponentFailed error");
    };

    assert_eq!(component_handle, ComponentHandle::from_str("test").unwrap());

    let RunComponentError::RunCallReturnedError { message, inner } = error else {
        panic!("Expected RunCallReturnedError error");
    };

    println!("message: {}", message);
    for inner in inner.iter() {
        println!("inner: {}", inner);
    }

    // This is testing that the full set of inner errors are propagated to the top level error,
    // so the user can reasonably see what went wrong.
    assert!(inner.len() == 2);
    assert_eq!(
        inner.last(),
        Some(&"invalid type: map, expected a sequence".to_string())
    );
}
