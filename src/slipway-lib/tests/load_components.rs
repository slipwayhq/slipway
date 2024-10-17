use serde_json::json;

use slipway_lib::{
    errors::{RigError, ValidationType},
    test_utils::{
        get_slipway_test_component_path, SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME,
        SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME, SLIPWAY_TEST_COMPONENT_NAME,
    },
    utils::ch,
    BasicComponentsLoader, ComponentCache, ComponentHandle, ComponentRigging, Instruction, Rig,
    RigSession, Rigging, SlipwayReference,
};

#[test]
fn load_component_from_folder() {
    test_component(SlipwayReference::Local {
        path: get_slipway_test_component_path(SLIPWAY_TEST_COMPONENT_NAME),
    });
}

#[test]
fn load_component_from_folder_with_json_schema_refs() {
    test_component(SlipwayReference::Local {
        path: get_slipway_test_component_path(SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME),
    });
}

#[test]
fn load_component_from_tar_with_json_schema_refs() {
    test_component(SlipwayReference::Local {
        path: get_slipway_test_component_path(SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME),
    });
}

fn create_rig(component_reference: SlipwayReference) -> (Rig, ComponentHandle) {
    let handle = ch("test_component");
    let rig = Rig::for_test(Rigging {
        components: [(
            handle.clone(),
            ComponentRigging {
                component: component_reference,
                input: Some(json! {{
                    "type": "increment",
                    "value": 42
                }}),
                permissions: None,
            },
        )]
        .into_iter()
        .collect(),
    });

    (rig, handle)
}

fn test_component(component_reference: SlipwayReference) {
    let (rig, handle) = create_rig(component_reference);

    let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::new()).unwrap();
    let rig_session = RigSession::new(rig, component_cache);
    let mut state = rig_session.initialize().unwrap();

    let good_input = json!({ "type": "increment", "value": 44});
    let bad_input = json!({ "type": "increment", "value": "bad"});
    let good_output = json!({ "value": 45});
    let bad_output = json!({ "value": "bad"});

    // Test invalid input
    let maybe_state = state.step(Instruction::SetInputOverride {
        handle: handle.clone(),
        value: bad_input.clone(),
    });

    match maybe_state {
        Err(RigError::ComponentValidationFailed {
            component_handle,
            validation_type,
            validation_failures: _,
            validated_data,
        }) => {
            assert_eq!(component_handle, handle);
            assert_eq!(validation_type, ValidationType::Input);
            assert_eq!(validated_data, bad_input);
        }
        _ => {
            panic!("Expected validation error");
        }
    }

    state = state
        .step(Instruction::SetInputOverride {
            handle: handle.clone(),
            value: good_input,
        })
        .unwrap();

    let maybe_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: bad_output.clone(),
    });

    match maybe_state {
        Err(RigError::ComponentValidationFailed {
            component_handle,
            validation_type,
            validation_failures: _,
            validated_data,
        }) => {
            assert_eq!(component_handle, handle);
            assert_eq!(validation_type, ValidationType::Output);
            assert_eq!(validated_data, bad_output);
        }
        _ => {
            panic!("Expected validation error");
        }
    }

    state
        .step(Instruction::SetOutput {
            handle: handle.clone(),
            value: good_output,
        })
        .unwrap();
}
