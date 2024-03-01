use std::collections::{HashMap, HashSet};

use crate::{
    errors::SlipwayError,
    parse::types::{primitives::ComponentHandle, App, ComponentRigging},
};

pub(crate) mod evaluate_component_inputs;
mod initialize;
mod primitives;
mod step;
mod topological_sort;

use primitives::Hash;

pub struct AppSession {
    app: App,
}

impl From<App> for AppSession {
    fn from(app: App) -> Self {
        AppSession { app }
    }
}

impl AppSession {
    pub fn initialize(&self) -> Result<AppExecutionState, SlipwayError> {
        initialize::initialize(self)
    }
}

pub struct AppExecutionState<'app> {
    session: &'app AppSession,
    component_states: HashMap<&'app ComponentHandle, ComponentState<'app>>,
    valid_execution_order: Vec<&'app ComponentHandle>,
    component_groups: Vec<HashSet<&'app ComponentHandle>>,
    wasm_cache: HashMap<&'app ComponentHandle, Vec<u8>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn step(
        self,
        instruction: step::Instruction,
    ) -> Result<AppExecutionState<'app>, SlipwayError> {
        step::step(self, instruction)
    }

    pub fn component_states(&self) -> &HashMap<&'app ComponentHandle, ComponentState> {
        &self.component_states
    }

    pub fn valid_execution_order(&self) -> &Vec<&'app ComponentHandle> {
        &self.valid_execution_order
    }

    pub fn component_groups(&self) -> &Vec<HashSet<&'app ComponentHandle>> {
        &self.component_groups
    }

    /// Internal because it returns a StepFailed error if the component does not exist.
    fn get_component_state_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Result<&mut ComponentState<'app>, SlipwayError> {
        let component_state =
            self.component_states
                .get_mut(handle)
                .ok_or(SlipwayError::StepFailed(format!(
                    "component {:?} does not exist in component states",
                    handle
                )))?;

        Ok(component_state)
    }

    /// Internal because it returns a StepFailed error if the component does not exist.
    fn get_component_state(
        &self,
        handle: &ComponentHandle,
    ) -> Result<&ComponentState<'app>, SlipwayError> {
        let component_state = self
            .component_states
            .get(handle)
            .ok_or(SlipwayError::StepFailed(format!(
                "component {:?} does not exist in component states",
                handle
            )))?;

        Ok(component_state)
    }
}

pub struct ComponentState<'app> {
    pub handle: &'app ComponentHandle,
    pub dependencies: HashSet<&'app ComponentHandle>,
    pub input_override: Option<ComponentInputOverride>,
    pub output_override: Option<ComponentOutputOverride>,
    pub execution_input: Option<ComponentInput>,
    pub execution_output: Option<ComponentOutput>,
}

impl<'app> ComponentState<'app> {
    /// Get the input of the component, which is either the input_override or the input or None.
    pub(crate) fn input(
        &self,
        component_rigging: &'app ComponentRigging,
    ) -> Option<&serde_json::Value> {
        match self.input_override.as_ref() {
            Some(input_override) => Some(&input_override.value),
            None => component_rigging.input.as_ref(),
        }
    }

    /// Get the output of the component, which is either the output_override or the execution_output or None.
    pub(crate) fn output(&self) -> Option<&serde_json::Value> {
        match self.output_override.as_ref() {
            Some(output_override) => Some(&output_override.value),
            None => self.execution_output.as_ref().map(|output| &output.value),
        }
    }
}

pub struct ComponentInput {
    pub value: serde_json::Value,
    pub hash: Hash,
}

pub struct ComponentInputOverride {
    pub value: serde_json::Value,
}

pub struct ComponentOutput {
    pub value: serde_json::Value,
    pub input_hash_used: Hash,
}

pub struct ComponentOutputOverride {
    pub value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        parse::types::{App, ComponentRigging, Rigging},
        utils::ch,
    };

    use super::{step::Instruction, *};

    fn assert_expected_components_ready(
        execution_state: &AppExecutionState,
        runnable_handles: &[&str],
    ) {
        for (handle, component_state) in execution_state.component_states() {
            let assert_ready = runnable_handles.contains(&handle.0.as_str());
            if assert_ready {
                if component_state.execution_input.is_none() {
                    panic!(
                        "expected component {:?} to have execution input",
                        handle.0.as_str()
                    );
                }

                if component_state.output().is_some() {
                    panic!(
                        "expected component {:?} to not have output",
                        handle.0.as_str()
                    );
                }
            } else if component_state.execution_input.is_some()
                && component_state.output().is_none()
            {
                panic!(
                "expected component {:?} not to be ready, but it has execution input and no output",
                handle.0.as_str()
            );
            }
        }
    }

    fn get_component_state<'app, 'local>(
        execution_state: &'local AppExecutionState<'app>,
        handle_str: &str,
    ) -> &'local ComponentState<'app> {
        let handle = ch(handle_str);
        execution_state.component_states.get(&handle).unwrap()
    }

    fn set_output_to<'a>(
        execution_state: AppExecutionState<'a>,
        next: &str,
        value: serde_json::Value,
    ) -> AppExecutionState<'a> {
        execution_state
            .step(Instruction::SetOutput {
                handle: ch(next),
                value,
            })
            .inspect_err(|e| println!("error: {:#}", e))
            .unwrap()
    }

    // Set the output of a component with a string of the same value as the component name.
    fn set_output<'a>(execution_state: AppExecutionState<'a>, next: &str) -> AppExecutionState<'a> {
        set_output_to(execution_state, next, json!(next))
    }

    mod step {
        use super::*;

        fn create_app() -> App {
            // Create a fully populated app instance.
            // Dependency graph:
            //     C
            //    /|\
            //   F B \
            //  / / \|
            // | E   A
            // | |   |
            // | |   D
            // | \  /
            //  \  G  I J
            //   \ | / /
            //     H -/  K
            App::for_test(Rigging {
                components: [
                    ComponentRigging::for_test("a", Some(json!({"b": "$$.b", "c": "$$.c"}))),
                    // "b" is used to test the chain e.input -> b.input -> c.output
                    ComponentRigging::for_test("b", Some(json!({"c": "$.rigging.c.output"}))),
                    // "c" is used to test reference to other parts of the app JSON.
                    ComponentRigging::for_test(
                        "c",
                        Some(json!({
                            "constant": "$.constants.test_constant",
                            "constant2": "$?constants.test_constant2",
                            "version": "$.version",
                        })),
                    ),
                    ComponentRigging::for_test(
                        "d",
                        Some(json!({ "foo": [ { "bar": { "a_x": "$$.a.x" } } ] })),
                    ),
                    // "e" is used to test the chain e.input -> b.input -> c.output
                    ComponentRigging::for_test(
                        "e",
                        Some(json!({
                            "b_input_y": "$?rigging.b.input.c.y",
                            "b_input_z": "$.rigging.b.input.c.z",
                        })),
                    ),
                    // "f" is used to test optional and required values.
                    ComponentRigging::for_test(
                        "f",
                        Some(json!({"c_x": "$$*c.x", "c_y": "$$?c.y", "c_z": "$$.c.z"})),
                    ),
                    ComponentRigging::for_test("g", Some(json!({"d": "$$.d", "e": "$$?e" }))),
                    ComponentRigging::for_test(
                        "h",
                        Some(json!({"g": "$$.g", "f": "$$.f", "i": "$$.i", "j": "$$.j" })),
                    ),
                    ComponentRigging::for_test("i", None),
                    ComponentRigging::for_test("j", Some(json!({"version": "$.version"}))),
                    ComponentRigging::for_test("k", None),
                ]
                .into_iter()
                .collect(),
            })
        }

        #[test]
        fn initialize_should_populate_execution_inputs_of_components_that_can_run_immediately() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let execution_state = app_session.initialize().unwrap();

            assert_expected_components_ready(&execution_state, &["c", "i", "j", "k"]);
        }

        #[test]
        fn it_should_populate_references_to_other_parts_of_app() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let s = app_session.initialize().unwrap();

            let c = get_component_state(&s, "c");

            assert_eq!(
                c.execution_input.as_ref().unwrap().value,
                json!({
                    "constant": "test_constant_value",
                    "constant2": null,
                    "version": "0.1.0"
                })
            );
        }

        #[test]
        fn it_should_allow_setting_the_output_on_a_component_which_can_execute() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "x": 1, "y": 2, "z": 3 }));
            assert_expected_components_ready(&s, &["f", "b", "i", "j", "k"]);

            let f = get_component_state(&s, "f");

            assert_eq!(
                f.execution_input.as_ref().unwrap().value,
                json!({ "c_x": [1], "c_y": 2, "c_z": 3 })
            );
        }

        #[test]
        fn it_should_not_allow_setting_the_output_on_a_component_which_cannot_execute() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let s = app_session.initialize().unwrap();

            let execution_state_result = s.step(Instruction::SetOutput {
                handle: ch("g"),
                value: json!({ "foo": "bar" }),
            });

            match execution_state_result {
                Ok(_) => panic!("expected an error"),
                Err(SlipwayError::StepFailed(s)) => {
                    assert_eq!(
                    s,
                    "component g cannot currently be executed, did you intend to override the output?"
                );
                }
                Err(err) => panic!("expected StepFailed error, got {:?}", err),
            }
        }

        #[test]
        fn it_should_allow_optional_json_path_references_missing_resolved_values() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "z": 3 }));

            assert_expected_components_ready(&s, &["f", "b", "i", "j", "k"]);

            let f = get_component_state(&s, "f");

            assert_eq!(
                f.execution_input.as_ref().unwrap().value,
                json!({ "c_x": [], "c_y": null, "c_z": 3 })
            );
        }

        #[test]
        fn it_should_not_allow_required_json_path_references_missing_resolved_values() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let s = app_session.initialize().unwrap();

            let execution_state_result = s.step(Instruction::SetOutput {
                handle: ch("c"),
                value: json!({ "x": 1, "y": 2 }),
            });

            match execution_state_result {
                Ok(_) => panic!("expected an error"),
                Err(SlipwayError::ResolveJsonPathFailed { message, state: _ }) => {
                    assert_eq!(
                        message,
                        r#"The input path "f.input.c_z" required "$.rigging.c.output.z" to be a value"#
                    );
                }
                Err(err) => panic!("expected StepFailed error, got {:?}", err),
            }
        }

        #[test]
        fn it_should_resolve_references_to_other_inputs_using_the_resolved_referenced_input() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "z": 3 }));
            s = set_output_to(s, "b", json!(null));

            assert_expected_components_ready(&s, &["f", "e", "a", "i", "j", "k"]);

            let e = get_component_state(&s, "e");

            assert_eq!(
                e.execution_input.as_ref().unwrap().value,
                json!({ "b_input_y": null, "b_input_z": 3 })
            );
        }

        #[test]
        fn it_should_step_though_entire_graph() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            assert_expected_components_ready(&s, &["c", "i", "j", "k"]);

            s = set_output_to(s, "c", json!({ "z": 3 }));

            assert_expected_components_ready(&s, &["f", "b", "i", "j", "k"]);

            s = set_output(s, "b");
            assert_expected_components_ready(&s, &["f", "e", "a", "i", "j", "k"]);

            s = set_output(s, "f");
            assert_expected_components_ready(&s, &["e", "a", "i", "j", "k"]);

            s = set_output(s, "k");
            assert_expected_components_ready(&s, &["e", "a", "i", "j"]);

            s = set_output(s, "j");
            assert_expected_components_ready(&s, &["e", "a", "i"]);

            s = set_output(s, "e");
            assert_expected_components_ready(&s, &["a", "i"]);

            s = set_output_to(s, "a", json!({ "x": 5 }));
            assert_expected_components_ready(&s, &["d", "i"]);

            s = set_output(s, "d");
            assert_expected_components_ready(&s, &["g", "i"]);

            s = set_output(s, "g");
            assert_expected_components_ready(&s, &["i"]);

            s = set_output(s, "i");
            assert_expected_components_ready(&s, &["h"]);

            s = set_output(s, "h");
            assert_expected_components_ready(&s, &[]);
        }
    }

    mod input_override {
        use itertools::Itertools;

        use super::*;

        fn create_app() -> App {
            // Create a fully populated app instance.
            // Dependency graph:
            //  C   D
            //  |
            //  B
            //  |
            //  A
            App::for_test(Rigging {
                components: [
                    ComponentRigging::for_test("a", Some(json!({ "b": "$$.b" }))),
                    ComponentRigging::for_test("b", Some(json!({ "c": "$$.c" }))),
                    ComponentRigging::for_test("c", None),
                    ComponentRigging::for_test("d", None),
                ]
                .into_iter()
                .collect(),
            })
        }

        fn assert_dependencies(
            execution_state: &AppExecutionState,
            component_handle: &str,
            expected_dependencies: &[&str],
        ) {
            let component_state = get_component_state(execution_state, component_handle);

            let actual_dependencies: Vec<_> = component_state
                .dependencies
                .iter()
                .map(|h| h.0.clone())
                .sorted()
                .collect();
            let expected_dependencies: Vec<_> =
                expected_dependencies.iter().cloned().sorted().collect();

            assert_eq!(actual_dependencies, expected_dependencies);
        }

        fn assert_group(
            execution_state: &AppExecutionState,
            group_index: usize,
            expected_handles: &[&str],
        ) {
            let actual_handles: Vec<_> = execution_state
                .component_groups()
                .get(group_index)
                .unwrap()
                .iter()
                .map(|h| h.0.clone())
                .sorted()
                .collect();
            let expected_handles: Vec<_> = expected_handles.iter().cloned().sorted().collect();

            assert_eq!(actual_handles, expected_handles);
        }

        #[test]
        fn setting_input_override_should_affect_dependencies() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            assert_dependencies(&s, "a", &["b"]);
            assert_dependencies(&s, "b", &["c"]);
            assert_dependencies(&s, "c", &[]);
            assert_dependencies(&s, "d", &[]);

            assert_group(&s, 0, &["a", "b", "c"]);
            assert_group(&s, 1, &["d"]);

            assert_expected_components_ready(&s, &["c", "d"]);
            assert_eq!(
                s.valid_execution_order,
                vec![&ch("c"), &ch("d"), &ch("b"), &ch("a")]
            );

            // Change "c" to depend on the output of "d".
            s = s
                .step(Instruction::SetInputOverride {
                    handle: ch("c"),
                    value: json!({ "d": "$$.d" }),
                })
                .unwrap();

            assert_dependencies(&s, "a", &["b"]);
            assert_dependencies(&s, "b", &["c"]);
            assert_dependencies(&s, "c", &["d"]);
            assert_dependencies(&s, "d", &[]);

            assert_group(&s, 0, &["a", "b", "c", "d"]);

            assert_expected_components_ready(&s, &["d"]);
            assert_eq!(
                s.valid_execution_order,
                vec![&ch("d"), &ch("c"), &ch("b"), &ch("a")]
            );

            // Reset to the original state.
            s = s
                .step(Instruction::ClearInputOverride { handle: ch("c") })
                .unwrap();

            assert_expected_components_ready(&s, &["c", "d"]);
            assert_eq!(
                s.valid_execution_order,
                vec![&ch("c"), &ch("d"), &ch("b"), &ch("a")]
            );
        }

        #[test]
        fn setting_input_override_should_update_input_hash() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            // Set the output on "c".
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                })
                .unwrap();

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                assert!(b.execution_output.is_none());
                b.execution_input.as_ref().unwrap().hash.clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(b.execution_input.as_ref().unwrap().hash, b_input_hash);
                assert_eq!(
                    b.execution_input.as_ref().unwrap().hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }

            // Override "b" input.
            s = s
                .step(Instruction::SetInputOverride {
                    handle: ch("b"),
                    value: json!({ "a": "b" }),
                })
                .unwrap();

            let b_input_hash_2 = {
                let b = s.get_component_state(&ch("b")).unwrap();

                // Input and output hash should no longer match.
                assert_ne!(
                    b.execution_input.as_ref().unwrap().hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );

                b.execution_input.as_ref().unwrap().hash.clone()
            };

            // Input hash should have changed.
            assert_ne!(b_input_hash, b_input_hash_2);

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "cat" }),
                })
                .unwrap();

            {
                // Check input and output hashes match again.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert!(b.execution_output.is_some());
                assert_eq!(b.execution_input.as_ref().unwrap().hash, b_input_hash_2);
                assert_eq!(
                    b.execution_input.as_ref().unwrap().hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }
        }
    }

    mod output_override {
        use super::*;

        fn create_app() -> App {
            // Create a fully populated app instance.
            // Dependency graph:
            //  C
            //  |
            //  B
            //  |
            //  A
            App::for_test(Rigging {
                components: [
                    ComponentRigging::for_test("a", Some(json!({ "b": "$$.b" }))),
                    ComponentRigging::for_test("b", Some(json!({ "c": "$$.c" }))),
                    ComponentRigging::for_test("c", None),
                ]
                .into_iter()
                .collect(),
            })
        }

        #[test]
        fn setting_output_override_should_affect_execution_states() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            assert_expected_components_ready(&s, &["c"]);
            assert_eq!(s.valid_execution_order, vec![&ch("c"), &ch("b"), &ch("a")]);

            // Override "b" output to allow "a" to execute immediately.
            s = s
                .step(Instruction::SetOutputOverride {
                    handle: ch("b"),
                    value: json!({ "foo": "bar" }),
                })
                .unwrap();

            assert_expected_components_ready(&s, &["c", "a"]);
            assert_eq!(s.valid_execution_order, vec![&ch("c"), &ch("b"), &ch("a")]);

            // Reset to the original state.
            s = s
                .step(Instruction::ClearOutputOverride { handle: ch("b") })
                .unwrap();

            assert_expected_components_ready(&s, &["c"]);
            assert_eq!(s.valid_execution_order, vec![&ch("c"), &ch("b"), &ch("a")]);
        }

        #[test]
        fn setting_output_should_use_input_hash() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            // Set the output on "c"
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                })
                .unwrap();

            // Verify the input and output hashes match for "c".
            let c = s.get_component_state(&ch("c")).unwrap();
            assert_eq!(
                c.execution_input.as_ref().unwrap().hash,
                c.execution_output.as_ref().unwrap().input_hash_used
            );

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input.as_ref().unwrap().hash.clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(b.execution_input.as_ref().unwrap().hash, b_input_hash);
                assert_eq!(
                    b.execution_input.as_ref().unwrap().hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }
        }

        #[test]
        fn setting_output_should_update_dependent_input_hashes() {
            let app = create_app();

            let app_session = AppSession::from(app);

            let mut s = app_session.initialize().unwrap();

            assert_expected_components_ready(&s, &["c"]);
            assert_eq!(s.valid_execution_order, vec![&ch("c"), &ch("b"), &ch("a")]);

            // Set the output on "c".
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                })
                .unwrap();

            assert_expected_components_ready(&s, &["b"]);

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input.as_ref().unwrap().hash.clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(b.execution_input.as_ref().unwrap().hash, b_input_hash);
                assert_eq!(
                    b.execution_input.as_ref().unwrap().hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }

            // Change "c" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "baz" }),
                })
                .unwrap();

            let b_input_hash_2 = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input.as_ref().unwrap().hash.clone()
            };

            // Hashes should be different.
            assert_ne!(b_input_hash, b_input_hash_2);

            // Revert "c" output using output override.
            s = s
                .step(Instruction::SetOutputOverride {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                })
                .unwrap();

            let b_input_hash_3 = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input.as_ref().unwrap().hash.clone()
            };

            assert_ne!(b_input_hash_3, b_input_hash_2);
            assert_eq!(b_input_hash_3, b_input_hash);
        }
    }
}
