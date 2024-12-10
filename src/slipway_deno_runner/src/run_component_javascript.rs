use std::{sync::Arc, time::Instant};

use deno_core::{
    serde_v8,
    v8::{self},
    JsRuntime, RuntimeOptions,
};
use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};

use crate::{DenoComponentDefinition, DENO_COMPONENT_DEFINITION_FILE_NAME};

pub(super) fn run_component_javascript(
    input: &serde_json::Value,
    deno_definition: Arc<DenoComponentDefinition>,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunComponentError> {
    if deno_definition.scripts.is_empty() {
        return Err(RunComponentError::Other(format!(
            "No scripts specified in definition file \"{}\"",
            DENO_COMPONENT_DEFINITION_FILE_NAME
        )));
    }

    let prepare_component_start = Instant::now();
    let mut runtime = JsRuntime::new(RuntimeOptions::default());
    let prepare_component_duration = prepare_component_start.elapsed();

    let prepare_input_start = Instant::now();
    {
        let scope = &mut runtime.handle_scope();
        let context = scope.get_current_context();

        let v8_input =
            serde_v8::to_v8(scope, input).map_err(|e| RunComponentError::GenericError(e.into()))?;
        let global = context.global(scope);
        let input_key = v8::String::new(scope, "input").expect("Input key should be valid");
        global.set(scope, input_key.into(), v8_input);
    }
    let prepare_input_duration = prepare_input_start.elapsed();

    let call_start = Instant::now();
    let script_files = &deno_definition.scripts;
    let mut last_result = None;
    for script_file in script_files.iter().cloned() {
        let content = execution_context.files.get_text(&script_file)?;
        let static_name: &'static str = Box::leak(script_file.into_boxed_str()); // TODO: Remove this leak.
        last_result = Some(
            runtime
                .execute_script(static_name, content.as_ref().clone())
                .map_err(|e| RunComponentError::RunCallFailed { source: e })?,
        );
    }
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();
    let last_result = last_result.expect("At least one script should be executed");
    let scope = &mut runtime.handle_scope();
    let local = v8::Local::new(scope, last_result);
    let value = serde_v8::from_v8::<serde_json::Value>(scope, local);
    let process_output_duration = process_output_start.elapsed();

    match value {
        Ok(output) => Ok(RunComponentResult {
            output,
            metadata: RunMetadata {
                prepare_input_duration,
                prepare_component_duration,
                call_duration,
                process_output_duration,
            },
        }),
        Err(error) => Err(RunComponentError::RunCallFailed {
            source: error.into(),
        }),
    }
}
