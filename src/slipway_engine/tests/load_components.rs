use std::{path::PathBuf, str::FromStr};

use common_macros::slipway_test_async;
use serde_json::json;

use common_test_utils::{
    SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME, SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_FOLDER_NAME,
    SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_NAME, SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME,
    get_slipway_test_components_path, get_slipway_test_components_registry_url,
    test_server::TestServer,
};
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, ComponentHandle, ComponentRigging, Instruction,
    Rig, RigSession, Rigging, SlipwayReference,
    errors::{RigError, ValidationType},
    utils::ch,
};
use url::Url;

#[slipway_test_async]
async fn load_component_from_folder() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_INCREMENT_COMPONENT_FOLDER_NAME),
        },
    )
    .await;
}

#[slipway_test_async]
async fn load_component_from_folder_with_json_schema_refs() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_FOLDER_NAME),
        },
    )
    .await;
}

#[slipway_test_async]
async fn load_component_from_tar_with_json_schema_refs() {
    test_component(
        None,
        SlipwayReference::Local {
            path: PathBuf::from(SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME),
        },
    )
    .await;
}

mod serial_tests {
    use super::*;

    #[slipway_test_async]
    async fn load_component_from_url_with_json_schema_refs() {
        let test_server = TestServer::start_from_folder(get_slipway_test_components_path());

        test_component(
            Some(&test_server.localhost_url),
            SlipwayReference::Http {
                url: Url::parse(&format!(
                    "{}{}",
                    test_server.localhost_url, SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_TAR_NAME
                ))
                .unwrap(),
            },
        )
        .await;

        test_server.stop();
    }

    #[slipway_test_async]
    async fn load_component_from_registry_with_json_schema_refs() {
        let test_server = TestServer::start_from_folder(get_slipway_test_components_path());

        let reference =
            SlipwayReference::from_str(SLIPWAY_INCREMENT_JSON_SCHEMA_COMPONENT_NAME).unwrap();

        match reference {
            SlipwayReference::Registry {
                publisher: _,
                name: _,
                version: _,
            } => {}
            _ => panic!("Expected registry reference"),
        }

        test_component(Some(&test_server.localhost_url), reference).await;

        test_server.stop();
    }
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
                allow: None,
                deny: None,
                callouts: None,
            },
        )]
        .into_iter()
        .collect(),
    });

    (rig, handle)
}

async fn test_component(localhost_url: Option<&str>, component_reference: SlipwayReference) {
    let (rig, handle) = create_rig(component_reference);

    // Use a random cache directory, and local registry URL.
    let temp_dir = tempfile::tempdir().unwrap();
    let component_cache = BasicComponentCache::primed(
        &rig,
        &BasicComponentsLoader::builder()
            .local_base_directory(&get_slipway_test_components_path())
            .registry_lookup_url(&get_slipway_test_components_registry_url())
            .registry_lookup_url(&format!(
                "{}{{publisher}}.{{name}}.{{version}}.tar",
                localhost_url.unwrap_or("http://localhost/")
            ))
            .without_default_registry()
            .components_cache_path(temp_dir.path())
            .build(),
    )
    .await
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
            assert_eq!(*validated_data, bad_input);
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
            assert_eq!(*validated_data, bad_output);
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
