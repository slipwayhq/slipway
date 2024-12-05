use std::{path::PathBuf, str::FromStr};

use serde_json::json;

use common_test_utils::{
    get_slipway_test_components_path, test_server::TestServer,
    SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME, SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME,
    SLIPWAY_TEST_COMPONENT_NAME,
};
use slipway_engine::{
    errors::{RigError, ValidationType},
    utils::ch,
    BasicComponentsLoader, ComponentCache, ComponentHandle, ComponentRigging, Instruction, Rig,
    RigSession, Rigging, SlipwayReference,
};
use url::Url;

#[test]
fn load_component_from_folder() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_TEST_COMPONENT_NAME),
        },
    );
}

#[test]
fn load_component_from_folder_with_json_schema_refs() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME),
        },
    );
}

#[test]
fn load_component_from_tar_with_json_schema_refs() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME),
        },
    );
}

#[test]
fn load_component_from_url_with_json_schema_refs() {
    let test_server = TestServer::start_from_folder(get_slipway_test_components_path());

    test_component(
        Some(&test_server.localhost_url),
        SlipwayReference::Url {
            url: Url::parse(&format!(
                "{}{}",
                test_server.localhost_url, SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_TAR_NAME
            ))
            .unwrap(),
        },
    );

    test_server.stop();
}

#[test]
fn load_component_from_registry_with_json_schema_refs() {
    let test_server = TestServer::start_from_folder(get_slipway_test_components_path());

    let reference = SlipwayReference::from_str(SLIPWAY_TEST_COMPONENT_JSON_SCHEMA_NAME).unwrap();

    match reference {
        SlipwayReference::Registry {
            publisher: _,
            name: _,
            version: _,
        } => {}
        _ => panic!("Expected registry reference"),
    }

    test_component(Some(&test_server.localhost_url), reference);

    test_server.stop();
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
                callouts: None,
            },
        )]
        .into_iter()
        .collect(),
    });

    (rig, handle)
}

fn test_component(localhost_url: Option<&str>, component_reference: SlipwayReference) {
    let (rig, handle) = create_rig(component_reference);

    // Use a random cache directory, and local registry URL.
    let temp_dir = tempfile::tempdir().unwrap();
    let component_cache = ComponentCache::primed(
        &rig,
        &BasicComponentsLoader::builder()
            .local_base_directory(&get_slipway_test_components_path())
            .registry_lookup_url(&format!(
                "{}{{publisher}}.{{name}}.{{version}}.tar",
                localhost_url.unwrap_or("http://localhost/")
            ))
            .components_cache_path(temp_dir.path())
            .build(),
    )
    .unwrap();

    // Initialize the rig session.
    let rig_session = RigSession::new(rig, &component_cache);
    let mut state = rig_session.initialize().unwrap();

    let good_input = json!({ "type": "increment", "value": 44});
    let bad_input = json!({ "type": "increment", "value": "bad"});
    let good_output = json!({ "value": 45});
    let bad_output = json!({ "value": "bad"});

    // Test invalid input.
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

    // Test valid input.
    state = state
        .step(Instruction::SetInputOverride {
            handle: handle.clone(),
            value: good_input,
        })
        .unwrap();

    // Test invalid output.
    let maybe_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: bad_output.clone(),
        metadata: Default::default(),
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

    // Test valid output.
    state
        .step(Instruction::SetOutput {
            handle: handle.clone(),
            value: good_output,
            metadata: Default::default(),
        })
        .unwrap();
}
