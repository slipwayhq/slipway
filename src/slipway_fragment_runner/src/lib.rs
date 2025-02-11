use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use slipway_engine::{
    prime_special_component, BasicComponentCache, Component, ComponentExecutionContext,
    ComponentExecutionData, ComponentHandle, ComponentRigging, ComponentRunner,
    MultiComponentCache, Rig, RigSession, Rigging, RunComponentError, RunComponentResult, Schema,
    SlipwayReference, SpecialComponentReference, TryRunComponentResult,
};
use slipway_host::run::{no_event_handler, run_rig};
use tracing::Instrument;

pub const FRAGMENT_COMPONENT_RUNNER_IDENTIFIER: &str = "fragment";
pub const INPUT_COMPONENT_HANDLE: &str = "input";
pub const OUTPUT_COMPONENT_HANDLE: &str = "output";

pub struct FragmentComponentRunner {}

#[async_trait(?Send)]
impl ComponentRunner for FragmentComponentRunner {
    fn identifier(&self) -> String {
        FRAGMENT_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn run<'call>(
        &self,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let component_definition = &execution_data.context.component_definition;

        let Some(rigging) = component_definition.rigging.as_ref() else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_fragment(
            input,
            Arc::clone(component_definition),
            rigging,
            &execution_data.context,
        )
        .instrument(tracing::info_span!("fragment"))
        .await?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

async fn run_component_fragment(
    input: &serde_json::Value,
    component_definition: Arc<Component<Schema>>,
    rigging: &Rigging,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunComponentError> {
    let input_component_handle = ComponentHandle::from_str(INPUT_COMPONENT_HANDLE)
        .expect("Default input component handle should be valid.");

    let output_component_handle = ComponentHandle::from_str(OUTPUT_COMPONENT_HANDLE)
        .expect("Default output component handle should be valid.");

    if rigging.components.contains_key(&input_component_handle) {
        return Err(RunComponentError::Other(format!(
            "Fragment should not contain a component with the handle \"{}\".",
            INPUT_COMPONENT_HANDLE
        )));
    }

    if !rigging.components.contains_key(&output_component_handle) {
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
            allow: None,
            deny: None,
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
    let call_chain = Arc::clone(&execution_context.call_chain);

    let original_component_cache = execution_context.component_cache;
    let new_component_cache = get_component_cache_with_pass_component();
    let component_cache =
        MultiComponentCache::new(vec![original_component_cache, &new_component_cache]);

    let rig_session = RigSession::new(rig, &component_cache);

    let run_result = run_rig(
        &rig_session,
        &mut no_event_handler(),
        component_runners,
        call_chain,
    )
    .await
    .map_err(|e| RunComponentError::RunCallFailed { source: e.into() })?;

    let output_state = run_result
        .component_outputs
        .get(&output_component_handle)
        .expect("Output component should exist.");

    let Some(output) = output_state.as_ref() else {
        return Err(RunComponentError::Other(format!(
            "Component with handle \"{}\" did not have any output set after fragment execution.",
            INPUT_COMPONENT_HANDLE
        )));
    };

    let result = RunComponentResult {
        output: output.value.clone(),
        metadata: output.run_metadata.clone(),
    };

    Ok(result)
}

fn get_component_cache_with_pass_component() -> BasicComponentCache {
    let pass_reference = SpecialComponentReference::Pass;
    let pass_component = prime_special_component(&pass_reference);
    BasicComponentCache::for_primed(
        vec![(
            SlipwayReference::Special(pass_reference.clone()),
            pass_component,
        )]
        .into_iter()
        .collect(),
    )
}
