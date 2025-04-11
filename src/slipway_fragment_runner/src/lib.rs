use std::{str::FromStr, sync::Arc, time::Instant};

use async_trait::async_trait;
use slipway_engine::{
    BasicComponentCache, Component, ComponentExecutionContext, ComponentExecutionData,
    ComponentHandle, ComponentRigging, ComponentRunner, MultiComponentCache, Rig, RigSession,
    Rigging, RunComponentError, RunComponentResult, RunMetadata, Schema, SlipwayReference,
    SpecialComponentReference, TryRunComponentResult, prime_special_component,
};
use slipway_host::run::{run_rig, tracing_event_handler};
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
    let prepare_input_start = Instant::now();
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
            component: SlipwayReference::Special(SpecialComponentReference::Passthrough),
            input: Some(input.clone()),
            allow: None,
            deny: None,
            permissions_chain: None,
            callouts: None,
        },
    );

    let rig = Rig {
        description: None,
        constants: component_definition.constants.clone(),
        rigging: rigging_with_input,
    };

    let prepare_input_duration = prepare_input_start.elapsed();
    let prepare_component_start = Instant::now();

    let component_runners = execution_context.component_runners;
    let call_chain = Arc::clone(&execution_context.call_chain);

    let original_component_cache = execution_context.component_cache;
    let new_component_cache = get_component_cache_with_pass_component().await;
    let component_cache =
        MultiComponentCache::new(vec![original_component_cache, &new_component_cache]);

    let rig_session = RigSession::new_with_options(
        rig,
        &component_cache,
        execution_context.rig_session_options.clone(),
    );

    let prepare_component_duration = prepare_component_start.elapsed();
    let call_start = Instant::now();

    let run_result = run_rig::<std::io::Error>(
        &rig_session,
        &mut tracing_event_handler(),
        component_runners,
        call_chain,
    )
    .await
    .map_err(|e| RunComponentError::RunCallFailed { source: e.into() })?;

    let call_duration = call_start.elapsed();
    let process_output_start = Instant::now();

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

    let output = output.value.clone();

    let process_output_duration = process_output_start.elapsed();

    let result = RunComponentResult {
        output,
        metadata: RunMetadata {
            prepare_input_duration,
            prepare_component_duration,
            call_duration,
            process_output_duration,
        },
    };

    Ok(result)
}

async fn get_component_cache_with_pass_component() -> BasicComponentCache {
    let pass_reference = SpecialComponentReference::Passthrough;
    let pass_component = prime_special_component(&pass_reference).await;
    BasicComponentCache::for_primed(
        vec![(
            SlipwayReference::Special(pass_reference.clone()),
            pass_component,
        )]
        .into_iter()
        .collect(),
    )
}
