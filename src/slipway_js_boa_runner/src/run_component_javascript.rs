use std::{sync::Arc, time::Instant};

use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};

use boa_engine::{
    builtins::promise::PromiseState, js_string, property::Attribute, Context, JsError, JsValue,
    Script, Source,
};
use slipway_host::ComponentError;
use tracing::debug;

use crate::{
    host::{prepare_canopy_host, SlipwayHost},
    BoaComponentDefinition, BOA_COMPONENT_DEFINITION_FILE_NAME,
};

pub(super) async fn run_component_javascript(
    input: &serde_json::Value,
    boa_definition: Arc<BoaComponentDefinition>,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunComponentError> {
    if boa_definition.scripts.is_empty() {
        return Err(RunComponentError::Other(format!(
            "No scripts specified in definition file \"{}\"",
            BOA_COMPONENT_DEFINITION_FILE_NAME
        )));
    }

    let prepare_component_start = Instant::now();
    let host = SlipwayHost::new(execution_context);
    let mut context = super::boa_environment::prepare_environment()?;
    prepare_canopy_host(&host, &mut context)?;
    let prepare_component_duration = prepare_component_start.elapsed();

    let prepare_input_start = Instant::now();
    set_input(&mut context, input)?;
    let prepare_input_duration = prepare_input_start.elapsed();

    let call_start = Instant::now();
    let last_result =
        run_component_scripts(&boa_definition.scripts, execution_context, &mut context).await?;
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();
    let output = convert_output(&mut context, last_result)?;
    let process_output_duration = process_output_start.elapsed();

    Ok(RunComponentResult {
        output,
        metadata: RunMetadata {
            prepare_input_duration,
            prepare_component_duration,
            call_duration,
            process_output_duration,
        },
    })
}

fn set_input(context: &mut Context, input: &serde_json::Value) -> Result<(), RunComponentError> {
    let value = JsValue::from_json(input, context)
        .map_err(|e| RunComponentError::Other(format!("Failed to convert input object.\n{}", e)))?;

    context
        .register_global_property(js_string!("input"), value, Attribute::default())
        .expect("input property shouldn't exist");

    Ok(())
}

async fn run_component_scripts(
    script_files: &[String],
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    context: &mut Context,
) -> Result<JsValue, RunComponentError> {
    let mut last_result = None;
    for script_file in script_files.iter() {
        let content = execution_context.files.get_text(script_file)?;

        debug!(
            "Running script \"{}\" ({} bytes) using Boa",
            script_file,
            content.len()
        );

        let script = Script::parse(Source::from_bytes(&*content), None, context)
            .map_err(|e| convert_error(script_file, context, e))?;

        last_result = Some(
            script
                .evaluate_async(context)
                .await
                .map_err(|e| convert_error(script_file, context, e))?,
        );
    }

    let last_result = last_result.expect("At least one script should be executed");

    context
        .run_jobs_async()
        .await
        .map_err(|e| RunComponentError::Other(format!("Failed to run async jobs\n{}", e)))?;

    let promise = last_result.as_promise();

    match promise {
        Some(promise) => match promise.state() {
            PromiseState::Pending => Err(RunComponentError::RunCallFailed {
                source: anyhow::anyhow!("Promise is still pending"),
            }),
            PromiseState::Fulfilled(result) => Ok(result),
            PromiseState::Rejected(error) => Err(convert_error(
                script_files
                    .last()
                    .expect("At least one script file should exist"),
                context,
                JsError::from_opaque(error),
            )),
        },
        None => Ok(last_result),
    }
}

fn convert_output(
    context: &mut Context,
    last_result: JsValue,
) -> Result<serde_json::Value, RunComponentError> {
    if last_result.is_undefined() {
        Ok(serde_json::Value::Null)
    } else {
        last_result.to_json(context).map_err(|e| {
            RunComponentError::Other(format!("Failed to convert output object.\n{}", e))
        })
    }
}

fn convert_error(script_file: &str, context: &mut Context, error: JsError) -> RunComponentError {
    let mut messages = Vec::new();
    let mut inner = Some(&error);
    while let Some(e) = inner {
        if let Some(native) = e.as_native() {
            messages.push(native.message().to_string());
            inner = native.cause();
        } else if let Some(opaque) = e.as_opaque() {
            let maybe_json = opaque.to_json(context);
            if let Ok(json) = maybe_json {
                let maybe_component_error = serde_json::from_value::<ComponentError>(json.clone());
                if let Ok(component_error) = maybe_component_error {
                    messages.push(component_error.message);
                    messages.extend(component_error.inner);
                } else if let Some(s) = json.as_str() {
                    messages.push(s.to_string());
                } else if let Some(o) = json.as_object() {
                    if let Some(message) = o.get("message").and_then(|v| v.as_str()) {
                        messages.push(message.to_string());
                    } else {
                        messages.push(format!("Unrecognized error: {:#?}", opaque));
                    }
                } else {
                    messages.push(format!("Unrecognized error: {:#?}", opaque));
                }
            } else {
                messages.push(format!(
                    "Failed to convert error object to JSON: {:#?}",
                    opaque
                ));
            };

            inner = None;
        } else {
            panic!("unexpected error type from Boa: {:?}", e);
        }
    }

    RunComponentError::RunCallReturnedError {
        message: format!("Failed to run script \"{}\"", script_file),
        inner: messages,
    }
}
