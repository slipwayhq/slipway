pub(crate) mod component_execution_data;
pub(crate) mod component_runner;
pub(crate) mod component_state;
mod evaluate_component_inputs;
pub(crate) mod fonts;
mod initialize;
pub(crate) mod primitives;
pub(crate) mod rig_execution_state;
pub(crate) mod rig_session;
mod run_record;
pub(crate) mod step;
mod topological_sort;
mod validate_component_io;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        ComponentState, Immutable, RigExecutionState,
        parse::types::{ComponentRigging, Rig, Rigging},
        utils::ch,
    };

    use super::step::Instruction;

    fn assert_expected_components_ready(
        execution_state: &RigExecutionState,
        runnable_handles: &[&str],
    ) {
        for (handle, component_state) in execution_state.component_states.iter() {
            let assert_ready = runnable_handles.contains(&handle.0.as_str());
            if assert_ready {
                if component_state.execution_input.is_none() {
                    panic!("Expected component \"{}\" to have execution input", handle);
                }

                if component_state.output().is_some() {
                    panic!("expected component \"{}\" to not have output", handle);
                }
            } else if component_state.execution_input.is_some()
                && component_state.output().is_none()
            {
                panic!(
                    "expected component \"{}\" not to be ready, but it has execution input and no output",
                    handle
                );
            }
        }
    }

    fn get_component_state<'rig, 'local>(
        execution_state: &'local RigExecutionState<'rig, '_>,
        handle_str: &str,
    ) -> &'local ComponentState<'rig> {
        let handle = ch(handle_str);
        execution_state.component_states.get(&handle).unwrap()
    }

    fn set_output_to<'rig, 'cache>(
        execution_state: Immutable<RigExecutionState<'rig, 'cache>>,
        next: &str,
        value: serde_json::Value,
    ) -> Immutable<RigExecutionState<'rig, 'cache>> {
        execution_state
            .step(Instruction::SetOutput {
                handle: ch(next),
                value,
                metadata: Default::default(),
            })
            .inspect_err(|e| println!("error: {:#}", e))
            .unwrap()
    }

    // Set the output of a component with a string of the same value as the component name.
    fn set_output<'rig, 'cache>(
        execution_state: Immutable<RigExecutionState<'rig, 'cache>>,
        next: &str,
    ) -> Immutable<RigExecutionState<'rig, 'cache>> {
        set_output_to(execution_state, next, json!(next))
    }

    mod step {
        use common_macros::slipway_test_async;

        use crate::{BasicComponentCache, RigSession, errors::RigError};

        use super::*;

        fn create_rig() -> Rig {
            // Create a fully populated rig instance.
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
            Rig::for_test(Rigging {
                components: [
                    ComponentRigging::for_test("a", Some(json!({"b": "$$.b", "c": "$$.c"}))),
                    // "b" is used to test the chain e.input -> b.input -> c.output
                    ComponentRigging::for_test("b", Some(json!({"c": "$.rigging.c.output"}))),
                    // "c" is used to test reference to other parts of the rig JSON.
                    ComponentRigging::for_test(
                        "c",
                        Some(json!({
                            "constant": "$.constants.test_constant",
                            "constant2": "$?constants.test_constant2",
                            "description": "$.description",
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
                    ComponentRigging::for_test("j", Some(json!({"description": "$.description"}))),
                    ComponentRigging::for_test("k", None),
                ]
                .into_iter()
                .collect(),
            })
        }

        #[slipway_test_async]
        async fn initialize_should_populate_execution_inputs_of_components_that_can_run_immediately()
         {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let execution_state = rig_session.initialize().unwrap();

            assert_expected_components_ready(&execution_state, &["c", "i", "j", "k"]);
        }

        #[slipway_test_async]
        async fn it_should_populate_references_to_other_parts_of_rig() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let s = rig_session.initialize().unwrap();

            let c = get_component_state(&s, "c");

            assert_eq!(
                c.execution_input.as_ref().unwrap().value,
                json!({
                    "constant": "test_constant_value",
                    "description": "test_description"
                })
            );
        }

        #[slipway_test_async]
        async fn it_should_allow_setting_the_output_on_a_component_which_can_execute() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "x": 1, "y": 2, "z": 3 }));
            assert_expected_components_ready(&s, &["f", "b", "i", "j", "k"]);

            let f = get_component_state(&s, "f");

            assert_eq!(
                f.execution_input.as_ref().unwrap().value,
                json!({ "c_x": [1], "c_y": 2, "c_z": 3 })
            );
        }

        #[slipway_test_async]
        async fn it_should_not_allow_setting_the_output_on_a_component_which_cannot_execute() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let s = rig_session.initialize().unwrap();

            let execution_state_result = s.step(Instruction::SetOutput {
                handle: ch("g"),
                value: json!({ "foo": "bar" }),
                metadata: Default::default(),
            });

            match execution_state_result {
                Ok(_) => panic!("expected an error"),
                Err(RigError::StepFailed { error }) => {
                    assert_eq!(
                        error,
                        "component g cannot currently be executed, did you intend to override the output?"
                    );
                }
                Err(err) => panic!("expected StepFailed error, got {}", err),
            }
        }

        #[slipway_test_async]
        async fn it_should_allow_optional_json_path_references_missing_resolved_values() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "z": 3 }));

            assert_expected_components_ready(&s, &["f", "b", "i", "j", "k"]);

            let f = get_component_state(&s, "f");

            assert_eq!(
                f.execution_input.as_ref().unwrap().value,
                json!({ "c_x": [], "c_z": 3 })
            );
        }

        #[slipway_test_async]
        async fn it_should_not_allow_required_json_path_references_missing_resolved_values() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let s = rig_session.initialize().unwrap();

            let execution_state_result = s.step(Instruction::SetOutput {
                handle: ch("c"),
                value: json!({ "x": 1, "y": 2 }),
                metadata: Default::default(),
            });

            match execution_state_result {
                Ok(_) => panic!("expected an error"),
                Err(RigError::ResolveJsonPathFailed { message, state: _ }) => {
                    assert_eq!(
                        message,
                        r#"The input path "f.input.c_z" required "$.rigging.c.output.z" to be a value"#
                    );
                }
                Err(err) => panic!("expected StepFailed error, got {}", err),
            }
        }

        #[slipway_test_async]
        async fn it_should_resolve_references_to_other_inputs_using_the_resolved_referenced_input()
        {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            s = set_output_to(s, "c", json!({ "z": 3 }));
            s = set_output_to(s, "b", json!(null));

            assert_expected_components_ready(&s, &["f", "e", "a", "i", "j", "k"]);

            let e = get_component_state(&s, "e");

            assert_eq!(
                e.execution_input.as_ref().unwrap().value,
                json!({ "b_input_z": 3 })
            );
        }

        #[slipway_test_async]
        async fn it_should_step_though_entire_graph() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

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
        use common_macros::slipway_test_async;

        use crate::{BasicComponentCache, RigSession, custom_iter_tools::CustomIterTools};

        use super::*;

        fn create_rig() -> Rig {
            // Create a fully populated rig instance.
            // Dependency graph:
            //  C   D
            //  |
            //  B
            //  |
            //  A
            Rig::for_test(Rigging {
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
            execution_state: &RigExecutionState,
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
            execution_state: &RigExecutionState,
            group_index: usize,
            expected_handles: &[&str],
        ) {
            let actual_handles: Vec<_> = execution_state
                .component_groups
                .get(group_index)
                .unwrap()
                .iter()
                .map(|h| h.0.clone())
                .sorted()
                .collect();
            let expected_handles: Vec<_> = expected_handles.iter().cloned().sorted().collect();

            assert_eq!(actual_handles, expected_handles);
        }

        #[slipway_test_async]
        async fn setting_input_override_should_affect_dependencies() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

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

        #[slipway_test_async]
        async fn setting_input_override_should_update_input_hash() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            // Set the output on "c".
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                    metadata: Default::default(),
                })
                .unwrap();

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                assert!(b.execution_output.is_none());
                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                    metadata: Default::default(),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b_input_hash
                );
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
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
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );

                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
            };

            // Input hash should have changed.
            assert_ne!(b_input_hash, b_input_hash_2);

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "cat" }),
                    metadata: Default::default(),
                })
                .unwrap();

            {
                // Check input and output hashes match again.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert!(b.execution_output.is_some());
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b_input_hash_2
                );
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }
        }
    }

    mod output_override {
        use common_macros::slipway_test_async;

        use crate::{BasicComponentCache, RigSession};

        use super::*;

        fn create_rig() -> Rig {
            // Create a fully populated rig instance.
            // Dependency graph:
            //  C
            //  |
            //  B
            //  |
            //  A
            Rig::for_test(Rigging {
                components: [
                    ComponentRigging::for_test("a", Some(json!({ "b": "$$.b" }))),
                    ComponentRigging::for_test("b", Some(json!({ "c": "$$.c" }))),
                    ComponentRigging::for_test("c", None),
                ]
                .into_iter()
                .collect(),
            })
        }

        #[slipway_test_async]
        async fn setting_output_override_should_affect_execution_states() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

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

        #[slipway_test_async]
        async fn setting_output_should_use_input_hash() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            // Set the output on "c"
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                    metadata: Default::default(),
                })
                .unwrap();

            // Verify the input and output hashes match for "c".
            let c = s.get_component_state(&ch("c")).unwrap();
            assert_eq!(
                c.execution_input.as_ref().unwrap().json_metadata.hash,
                c.execution_output.as_ref().unwrap().input_hash_used
            );

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                    metadata: Default::default(),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b_input_hash
                );
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }
        }

        #[slipway_test_async]
        async fn setting_output_should_update_dependent_input_hashes() {
            let rig = create_rig();

            let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
            let rig_session = RigSession::new_for_test(rig, &component_cache);

            let mut s = rig_session.initialize().unwrap();

            assert_expected_components_ready(&s, &["c"]);
            assert_eq!(s.valid_execution_order, vec![&ch("c"), &ch("b"), &ch("a")]);

            // Set the output on "c".
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "bar" }),
                    metadata: Default::default(),
                })
                .unwrap();

            assert_expected_components_ready(&s, &["b"]);

            // Save "b" input hash to compare against later.
            let b_input_hash = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
            };

            // Set "b" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("b"),
                    value: json!({ "baz": "bat" }),
                    metadata: Default::default(),
                })
                .unwrap();

            {
                // Check input and output hashes match.
                let b = s.get_component_state(&ch("b")).unwrap();
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b_input_hash
                );
                assert_eq!(
                    b.execution_input.as_ref().unwrap().json_metadata.hash,
                    b.execution_output.as_ref().unwrap().input_hash_used
                );
            }

            // Change "c" output.
            s = s
                .step(Instruction::SetOutput {
                    handle: ch("c"),
                    value: json!({ "foo": "baz" }),
                    metadata: Default::default(),
                })
                .unwrap();

            let b_input_hash_2 = {
                let b = s.get_component_state(&ch("b")).unwrap();
                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
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
                b.execution_input
                    .as_ref()
                    .unwrap()
                    .json_metadata
                    .hash
                    .clone()
            };

            assert_ne!(b_input_hash_3, b_input_hash_2);
            assert_eq!(b_input_hash_3, b_input_hash);
        }
    }
}
