use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use crate::run_component_wasm::run_component_wasm;

use super::errors::SlipwayDebugError;

pub(super) fn handle_run_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let execution_data = state.get_component_execution_data(handle)?;

    let output = run_component_wasm(execution_data, handle)?;

    let new_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: output,
    })?;

    Ok(new_state)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use serde_json::json;
    use slipway_lib::{
        utils::ch, App, AppSession, BasicComponentsLoader, ComponentCache, ComponentRigging,
        Rigging, SlipwayReference,
    };

    use crate::run_component_wasm::errors::WasmExecutionError;

    use super::*;

    fn find_ancestor_path(path_to_find: PathBuf) -> PathBuf {
        let mut current_path = std::env::current_dir().unwrap();

        let mut searched = Vec::new();
        loop {
            let current_search_path = current_path.join(&path_to_find);
            searched.push(current_search_path.clone());

            if current_search_path.exists() {
                return current_search_path;
            }

            if !current_path.pop() {
                panic!(
                    "Could not find ancestor path: {path_to_find:?}.\nSearched:\n{searched}\n",
                    searched = searched
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<String>>()
                        .join("\n")
                );
            }
        }
    }

    #[test]
    fn it_should_run_basic_component() {
        let handle = ch("test_component");
        let app = App::for_test(Rigging {
            components: [(
                handle.clone(),
                ComponentRigging {
                    component: SlipwayReference::Local {
                        path: find_ancestor_path(
                            PathBuf::from_str(
                                "wasm/target/wasm32-wasi/debug/slipway_test_component.json",
                            )
                            .unwrap(),
                        ),
                    },
                    input: Some(json!({ "type": "increment", "value": 42})),
                    permissions: None,
                },
            )]
            .into_iter()
            .collect(),
        });

        let component_cache = ComponentCache::primed(&app, &BasicComponentsLoader::new()).unwrap();
        let app_session = AppSession::new(app, component_cache);
        let mut state = app_session.initialize().unwrap();

        state = handle_run_command(&handle, &state).unwrap();

        let component_state = state
            .component_states
            .get(&handle)
            .expect("Component should exist");

        let maybe_execution_output = &component_state.execution_output;

        if let Some(execution_output) = maybe_execution_output {
            // The component will increment `value` by 1.
            assert_eq!(execution_output.value, json!({ "value": 43 }));
        } else {
            panic!("Component should have execution output");
        }
    }

    #[test]
    fn it_should_handle_component_that_panics() {
        let handle = ch("test_component");
        let app = App::for_test(Rigging {
            components: [(
                handle.clone(),
                ComponentRigging {
                    component: SlipwayReference::Local {
                        path: find_ancestor_path(
                            PathBuf::from_str(
                                "wasm/target/wasm32-wasi/debug/slipway_test_component.json",
                            )
                            .unwrap(),
                        ),
                    },
                    input: Some(json!({ "type": "panic" })),
                    permissions: None,
                },
            )]
            .into_iter()
            .collect(),
        });

        let component_cache = ComponentCache::primed(&app, &BasicComponentsLoader::new()).unwrap();
        let app_session = AppSession::new(app, component_cache);
        let state = app_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(WasmExecutionError::StepCallFailed(
                stderr_string,
                Some(_),
            ))) => {
                assert!(stderr_string.contains("slipway-test-component-panic"));
            }
            _ => panic!("Expected WasmExecutionError/StepCallFailed"),
        }
    }

    #[test]
    fn it_should_handle_component_that_errors() {
        let handle = ch("test_component");
        let app = App::for_test(Rigging {
            components: [(
                handle.clone(),
                ComponentRigging {
                    component: SlipwayReference::Local {
                        path: find_ancestor_path(
                            PathBuf::from_str(
                                "wasm/target/wasm32-wasi/debug/slipway_test_component.json",
                            )
                            .unwrap(),
                        ),
                    },
                    input: Some(json!({ "type": "stderr" })),
                    permissions: None,
                },
            )]
            .into_iter()
            .collect(),
        });

        let component_cache = ComponentCache::primed(&app, &BasicComponentsLoader::new()).unwrap();
        let app_session = AppSession::new(app, component_cache);
        let state = app_session.initialize().unwrap();

        let maybe_state = handle_run_command(&handle, &state);

        match maybe_state {
            Err(SlipwayDebugError::WasmExecutionFailed(WasmExecutionError::StepCallFailed(
                stderr_string,
                None,
            ))) => {
                assert_eq!(stderr_string, "slipway-test-component-stderr");
            }
            _ => panic!("Expected WasmExecutionError/StepCallFailed"),
        }
    }
}
