use std::collections::{HashMap, HashSet};

use crate::{
    errors::SlipwayError,
    parse::types::{primitives::ComponentHandle, App, ComponentRigging},
};

pub(crate) mod evaluate_component_inputs;
pub(crate) mod initialize;
mod primitives;
pub(crate) mod step;
mod topological_sort;

use primitives::Hash;

pub(crate) fn create_session(app: App) -> AppSession {
    AppSession { app }
}

pub use initialize::initialize;
pub use step::step;

fn get_component_state_mut<'app, 'local>(
    state: &'local mut AppExecutionState<'app>,
    handle: &'local ComponentHandle,
) -> Result<&'local mut crate::ComponentState<'app>, SlipwayError> {
    let component_state =
        state
            .component_states
            .get_mut(handle)
            .ok_or(SlipwayError::StepFailed(format!(
                "component {:?} does not exist in component states",
                handle
            )))?;

    Ok(component_state)
}

fn get_component_state<'app, 'local>(
    state: &'local AppExecutionState<'app>,
    handle: &'local ComponentHandle,
) -> Result<&'local crate::ComponentState<'app>, SlipwayError> {
    let component_state = state
        .component_states
        .get(handle)
        .ok_or(SlipwayError::StepFailed(format!(
            "component {:?} does not exist in component states",
            handle
        )))?;

    Ok(component_state)
}

pub struct AppSession {
    app: App,
}

pub struct AppExecutionState<'app> {
    session: &'app AppSession,
    component_states: HashMap<&'app ComponentHandle, ComponentState<'app>>,
    execution_order: Vec<&'app ComponentHandle>,
    wasm_cache: HashMap<&'app ComponentHandle, Vec<u8>>,
}

impl<'app> AppExecutionState<'app> {
    pub fn component_states(&self) -> &HashMap<&'app ComponentHandle, ComponentState> {
        &self.component_states
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
    value: serde_json::Value,
    hash: Hash,
}

pub struct ComponentInputOverride {
    value: serde_json::Value,
}

pub struct ComponentOutput {
    value: serde_json::Value,
    input_hash_used: Hash,
}

pub struct ComponentOutputOverride {
    value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use semver::Version;
    use serde_json::{json, Value};

    use crate::{
        parse::types::{
            primitives::{Name, Publisher},
            slipway_reference::SlipwayReference,
            App, ComponentRigging, Rigging,
        },
        ComponentHandle,
    };

    use self::step::Instruction;

    use super::*;

    fn create_app() -> App {
        fn create_component(
            name: &str,
            input: Option<Value>,
        ) -> (ComponentHandle, ComponentRigging) {
            (
                ComponentHandle::from_str(name).unwrap(),
                ComponentRigging {
                    component: SlipwayReference::from_str(&format!("p{name}.{name}.0.1.0"))
                        .unwrap(),
                    input,
                    permissions: None,
                },
            )
        }

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
        App {
            publisher: Publisher::from_str("test_publisher").unwrap(),
            name: Name::from_str("test_name").unwrap(),
            version: Version::from_str("0.1.0").unwrap(),
            description: None,
            constants: Some(json!({"test_constant": "test_constant_value"})),
            rigging: Rigging {
                components: [
                    create_component("a", Some(json!({"b_x": "$$.b.x", "c": "$$.c"}))),
                    create_component("b", Some(json!({"c": "$.rigging.c.output"}))),
                    create_component("c", Some(json!({"constant": "$.constants.test_constant"}))),
                    create_component(
                        "d",
                        Some(json!({ "foo": [ { "bar": { "a_x": "$$?a.x" } } ] })),
                    ),
                    create_component("e", Some(json!({"b_bat": "$?rigging.b.output.baz[2].bat"}))),
                    create_component(
                        "f",
                        Some(json!({"c_x": "$$.c.x", "c_y": "$$?c.y", "c_z": "$$!c.z"})),
                    ),
                    create_component("g", Some(json!({"d": "$$.d", "e": "$$?e" }))),
                    create_component("h", Some(json!({"g": "$$.g", "f": "$$.f" }))),
                    create_component("i", None),
                    create_component("j", Some(json!({"version": "$!version"}))),
                    create_component("k", None),
                ]
                .into_iter()
                .collect(),
            },
        }
    }

    fn assert_expected_components_ready(
        execution_state: &AppExecutionState,
        runable_handles: &[&str],
    ) {
        for (handle, component_state) in execution_state.component_states() {
            let assert_ready = runable_handles.contains(&handle.0.as_str());
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

    fn get<'app, 'local>(
        execution_state: &'local AppExecutionState<'app>,
        handle_str: &str,
    ) -> &'local ComponentState<'app> {
        let handle = ComponentHandle::from_str(handle_str).unwrap();
        execution_state.component_states.get(&handle).unwrap()
    }

    #[test]
    fn initialize_should_populate_execution_inputs_of_components_that_can_run_immediately() {
        let app = create_app();

        let app_session = create_session(app);

        let execution_state = initialize(&app_session).unwrap();

        assert_expected_components_ready(&execution_state, &["c", "i", "j", "k"]);
    }

    #[test]
    fn it_should_handle_setting_the_output_on_a_component_which_can_execute() {
        let app = create_app();

        let app_session = create_session(app);

        let mut execution_state = initialize(&app_session).unwrap();

        execution_state = step(
            execution_state,
            Instruction::SetOutput {
                handle: ComponentHandle::from_str("c").unwrap(),
                value: json!({ "x": 1, "y": 2, "z": 3 }),
            },
        )
        .unwrap();

        assert_expected_components_ready(&execution_state, &["f", "b", "i", "j", "k"]);

        let f = get(&execution_state, "f");

        assert_eq!(
            f.execution_input.as_ref().unwrap().value,
            json!({ "c_x": [1], "c_y": 2, "c_z": 3 })
        );
    }

    #[test]
    fn it_should_error_when_setting_the_output_on_a_component_which_cannot_execute() {
        let app = create_app();

        let app_session = create_session(app);

        let execution_state = initialize(&app_session).unwrap();

        let execution_state_result = step(
            execution_state,
            Instruction::SetOutput {
                handle: ComponentHandle::from_str("g").unwrap(),
                value: json!({ "foo": "bar" }),
            },
        );

        match execution_state_result {
            Ok(_) => panic!("expected an error"),
            Err(SlipwayError::StepFailed(s)) => {
                assert_eq!(
                    s,
                    "component g cannot be executed, did you intend to override the output?"
                );
            }
            Err(err) => panic!("expected StepFailed error, got {:?}", err),
        }
    }

    #[test]
    fn it_should_handle_optional_values() {
        let app = create_app();

        let app_session = create_session(app);

        let mut execution_state = initialize(&app_session).unwrap();

        execution_state = step(
            execution_state,
            Instruction::SetOutput {
                handle: ComponentHandle::from_str("c").unwrap(),
                value: json!({ "z": 3 }),
            },
        )
        .unwrap();

        assert_expected_components_ready(&execution_state, &["f", "b", "i", "j", "k"]);

        let f = get(&execution_state, "f");

        assert_eq!(
            f.execution_input.as_ref().unwrap().value,
            json!({ "c_x": [], "c_y": null, "c_z": 3 })
        );
    }

    #[test]
    fn it_should_error_when_missing_required_values() {
        let app = create_app();

        let app_session = create_session(app);

        let execution_state = initialize(&app_session).unwrap();

        let execution_state_result = step(
            execution_state,
            Instruction::SetOutput {
                handle: ComponentHandle::from_str("c").unwrap(),
                value: json!({ "x": 1, "y": 2 }),
            },
        );

        match execution_state_result {
            Ok(_) => panic!("expected an error"),
            Err(SlipwayError::StepFailed(s)) => {
                assert_eq!(
                    s,
                    r#"The input path "$.c_z" required "$.rigging.c.output.z" to be a value"#
                );
            }
            Err(err) => panic!("expected StepFailed error, got {:?}", err),
        }

        todo!("the error message should include which component input failed")
    }
    // Test the case where a component references another component's input, and the
    // refrenced input contains a reference to another component's output.

    // If an input references somethign that does not exist, it should fail.
}
