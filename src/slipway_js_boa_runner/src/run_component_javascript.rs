use std::{path::Path, sync::Arc, time::Instant};

use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};

use boa_engine::{
    Context, JsError, JsValue, Module, Script, Source, builtins::promise::PromiseState, js_string,
};
use slipway_host::ComponentError;
use tracing::{debug, warn};

use crate::{
    BOA_RUN_JS_FILE_NAME, BoaComponentDefinition,
    host::{SlipwayHost, prepare_slipway_host},
};

pub(super) async fn run_component_javascript(
    input: &serde_json::Value,
    run_js: Arc<String>,
    boa_definition: Option<Arc<BoaComponentDefinition>>,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunComponentError> {
    let prepare_component_start = Instant::now();
    let host = SlipwayHost::new(execution_context);
    let mut context =
        super::boa_environment::prepare_environment(Arc::clone(&execution_context.files))?;
    prepare_slipway_host(&host, &mut context)?;
    let prepare_component_duration = prepare_component_start.elapsed();

    let prepare_input_start = Instant::now();
    let input = convert_input(&mut context, input)?;
    let prepare_input_duration = prepare_input_start.elapsed();

    let scripts = boa_definition
        .as_ref()
        .map(|def| def.scripts.as_slice())
        .unwrap_or_else(|| &[]);

    let call_start = Instant::now();
    let last_result =
        run_component_scripts_and_modules(&run_js, scripts, execution_context, &mut context, input)
            .await?;
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

fn convert_input(
    context: &mut Context,
    input: &serde_json::Value,
) -> Result<JsValue, RunComponentError> {
    let value = JsValue::from_json(input, context)
        .map_err(|e| RunComponentError::Other(format!("Failed to convert input object.\n{}", e)))?;
    Ok(value)
}

fn is_run_js(script_file: &str) -> bool {
    script_file == BOA_RUN_JS_FILE_NAME
        || (script_file.ends_with(BOA_RUN_JS_FILE_NAME)
            && script_file.starts_with("./")
            && script_file.len() == BOA_RUN_JS_FILE_NAME.len() + 2)
}

async fn run_component_scripts_and_modules(
    run_js: &str,
    script_files: &[String],
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    context: &mut Context,
    input: JsValue,
) -> Result<JsValue, RunComponentError> {
    for script_file in script_files.iter() {
        if is_run_js(script_file) {
            warn!(
                "Ignoring script \"{}\" as it is the component Javascript module. This should be removed from the scripts list.",
                script_file
            );
            continue;
        }

        let content = execution_context.files.get_text(script_file).await?;

        run_script(script_file, &content, context).await?;
    }

    debug!(
        "Running Javascript component: {}",
        execution_context.component_reference
    );

    let module = Module::parse(
        Source::from_bytes(&run_js).with_path(Path::new(BOA_RUN_JS_FILE_NAME)),
        None,
        context,
    )
    .map_err(|e| convert_error(BOA_RUN_JS_FILE_NAME, context, e))?;

    let module_promise = module.load_link_evaluate(context);

    context.run_jobs_async().await.map_err(|e| {
        RunComponentError::Other(format!(
            "Failed to run async jobs after loading scripts and {BOA_RUN_JS_FILE_NAME} module\n{}",
            e
        ))
    })?;

    match module_promise.state() {
        PromiseState::Pending => Err(RunComponentError::RunCallFailed {
            source: anyhow::anyhow!(
                "Promise from evaluating {BOA_RUN_JS_FILE_NAME} module is still pending"
            ),
        }),
        PromiseState::Fulfilled(_) => {
            // The module was evaluated successfully, so we can continue.
            Ok(())
        }
        PromiseState::Rejected(error) => Err(convert_error(
            BOA_RUN_JS_FILE_NAME,
            context,
            JsError::from_opaque(error),
        )),
    }?;

    let namespace = module.namespace(context);

    const RUN_FUNCTION_EXPORT_NAME: &str = "run";
    let run = namespace
        .get(js_string!(RUN_FUNCTION_EXPORT_NAME), context)
        .map_err(|e| convert_error(BOA_RUN_JS_FILE_NAME, context, e))?
        .as_callable()
        .cloned()
        .ok_or_else(|| {
            RunComponentError::Other(format!(
                "The export \"{RUN_FUNCTION_EXPORT_NAME}\" wasn't a function."
            ))
        })?;

    let script_name = format!("{BOA_RUN_JS_FILE_NAME}::{RUN_FUNCTION_EXPORT_NAME}");
    let result = run
        .call(&JsValue::undefined(), &[input], context)
        .map_err(|e| convert_error(&script_name, context, e))?;

    context
        .run_jobs_async()
        .await
        .map_err(|e| RunComponentError::Other(format!("Failed to run async jobs after executing {BOA_RUN_JS_FILE_NAME} {RUN_FUNCTION_EXPORT_NAME} function\n{}", e)))?;

    let promise = result.as_promise();
    match promise {
        Some(promise) => match promise.state() {
            PromiseState::Pending => Err(RunComponentError::RunCallFailed {
                source: anyhow::anyhow!(
                    "Output promise from {BOA_RUN_JS_FILE_NAME} {RUN_FUNCTION_EXPORT_NAME} function is still pending"
                ),
            }),
            PromiseState::Fulfilled(result) => Ok(result),
            PromiseState::Rejected(error) => Err(convert_error(
                &script_name,
                context,
                JsError::from_opaque(error),
            )),
        },
        None => Ok(result),
    }
}

async fn run_script(
    name: &str,
    content: &str,
    context: &mut Context,
) -> Result<JsValue, RunComponentError> {
    debug!(
        "Running script \"{}\" ({} bytes) using Boa",
        name,
        content.len()
    );

    let script = Script::parse(Source::from_bytes(&content), None, context)
        .map_err(|e| convert_error(name, context, e))?;

    script
        .evaluate_async(context)
        .await
        .map_err(|e| convert_error(name, context, e))
}

fn convert_output(
    context: &mut Context,
    last_result: JsValue,
) -> Result<serde_json::Value, RunComponentError> {
    if last_result.is_undefined() {
        Ok(serde_json::Value::Null)
    } else {
        last_result
            .to_json(context)
            .map(|v| match v {
                None => serde_json::Value::Null,
                Some(v) => v,
            })
            .map_err(|e| {
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
            let maybe_json = opaque.to_json(context).map(|v| match v {
                None => serde_json::Value::Null,
                Some(v) => v,
            });
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
