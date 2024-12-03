use std::{str::FromStr, sync::Arc};

use slipway_engine::{
    Component, ComponentExecutionContext, ComponentExecutionData, ComponentHandle,
    ComponentRigging, ComponentRunner, Rig, RigSession, Rigging, RunComponentError,
    RunComponentResult, Schema, SlipwayReference, SpecialComponentReference, TryRunComponentResult,
};
use slipway_host::run::{run_rig, RunEventHandler};

pub const FRAGMENT_COMPONENT_RUNNER_IDENTIFIER: &str = "fragment";
pub const INPUT_COMPONENT_HANDLE: &str = "input";
pub const OUTPUT_COMPONENT_HANDLE: &str = "output";

pub struct FragmentComponentRunner {}

impl<'rig> ComponentRunner<'rig> for FragmentComponentRunner {
    fn identifier(&self) -> String {
        FRAGMENT_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    fn run<'call, 'runners>(
        &self,
        handle: &'call ComponentHandle,
        execution_data: &'call ComponentExecutionData<'call, 'rig, 'runners>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let component_definition = execution_data.get_component_definition(handle);

        let Some(rigging) = component_definition.rigging.as_ref() else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_fragment(
            handle,
            input,
            Arc::clone(&component_definition),
            rigging,
            &execution_data.context,
        )?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

fn run_component_fragment(
    handle: &ComponentHandle,
    input: &serde_json::Value,
    component_definition: Arc<Component<Schema>>,
    rigging: &Rigging,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunComponentError> {
    let input_component_handle = ComponentHandle::from_str(INPUT_COMPONENT_HANDLE)
        .expect("Default input component handle should be valid.");

    let output_component_handle = ComponentHandle::from_str(OUTPUT_COMPONENT_HANDLE)
        .expect("Default output component handle should be valid.");

    if rigging.components.get(&input_component_handle).is_some() {
        return Err(RunComponentError::Other(format!(
            "Fragment should not contain a component with the handle \"{}\".",
            INPUT_COMPONENT_HANDLE
        )));
    }

    if rigging.components.get(&output_component_handle).is_none() {
        return Err(RunComponentError::Other(format!(
            "Fragment must contain a component with the handle \"{}\".",
            OUTPUT_COMPONENT_HANDLE
        )));
    }

    let mut rigging_with_input = rigging.clone();

    rigging_with_input.components.insert(
        input_component_handle,
        ComponentRigging {
            component: SlipwayReference::Special(SpecialComponentReference::Pass),
            input: Some(input.clone()),
            permissions: None,
            callouts: None,
        },
    );

    let rig = Rig {
        publisher: component_definition.publisher.clone(),
        name: component_definition.name.clone(),
        version: component_definition.version.clone(),
        description: None,
        constants: component_definition.constants.clone(),
        rigging: rigging_with_input,
    };

    let component_runners = execution_context.component_runners;
    let permission_chain = execution_context.permission_chain;

    let component_cache = execution_context.callout_context.component_cache;
    let rig_session = RigSession::new(rig, component_cache);

    run_rig(&rig_session, None, component_runners, permission_chain);
    // Handling input:
    // Perhaps create an "input" component and set the output on it.

    // Handling output:
    // 🤔
    // Take the first component with a dangling output.
    // A fragment should be built with only one component with a dangling output.
    // Or a component with the handle "output" perhaps.
}

// struct SinkRunEventHandler {}

// impl<'rig> RunEventHandler<'rig, ()> for SinkRunEventHandler {
//     fn handle_component_run_start(
//         &mut self,
//         _event: slipway_host::run::ComponentRunStartEvent<'rig>,
//     ) -> Result<(), ()> {
//         Ok(())
//     }

//     fn handle_component_run_end(
//         &mut self,
//         _event: slipway_host::run::ComponentRunEndEvent<'rig>,
//     ) -> Result<(), ()> {
//         Ok(())
//     }

//     fn handle_state_changed<'state>(
//         &mut self,
//         _event: slipway_host::run::StateChangeEvent<'rig, 'state>,
//     ) -> Result<(), ()> {
//         Ok(())
//     }
// }
