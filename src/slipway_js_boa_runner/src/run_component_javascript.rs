use std::{sync::Arc, time::Instant};

use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};

use boa_engine::{js_string, property::Attribute, Context, JsValue, Source};
use tracing::debug;

use crate::{BoaComponentDefinition, BOA_COMPONENT_DEFINITION_FILE_NAME};

pub(super) fn run_component_javascript(
    input: &serde_json::Value,
    boa_definition: Arc<BoaComponentDefinition>,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunComponentError> {
    if boa_definition.scripts.is_empty() {
        return Err(RunComponentError::Other(format!(
            "No scripts specified in definition file \"{}\"",
            BOA_COMPONENT_DEFINITION_FILE_NAME
        )));
    }

    let prepare_component_start = Instant::now();
    let mut context = Context::default();
    prepare_environment(&mut context)?;
    let prepare_component_duration = prepare_component_start.elapsed();

    let prepare_input_start = Instant::now();
    set_input(&mut context, input)?;
    restore_input_prototypes(&mut context)?;
    let prepare_input_duration = prepare_input_start.elapsed();

    let call_start = Instant::now();
    let last_result =
        run_component_scripts(&boa_definition.scripts, execution_context, &mut context)?;
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

fn prepare_environment(context: &mut Context) -> Result<(), RunComponentError> {
    context
        .eval(Source::from_bytes(
            r#"
            function argsToMessage(...args) {
                return args.map((arg) => JSON.stringify(arg)).join(" ");
            }

            console = {
                trace: (...args) => {
                    
                },
                debug: (...args) => {
                    
                },
                log: (...args) => {
                    
                },
                warn: (...args) => {
                    
                },
                error: (...args) => {
                    
                },
            };

            global = {};
            setTimeout = () => { };
            clearTimeout = () => { };
            "#,
        ))
        .map_err(|e| {
            RunComponentError::Other(format!("Failed to prepare javascript environment.\n{}", e))
        })?;
    Ok(())
}

fn set_input(context: &mut Context, input: &serde_json::Value) -> Result<(), RunComponentError> {
    let value = JsValue::from_json(input, context)
        .map_err(|e| RunComponentError::Other(format!("Failed to convert input object.\n{}", e)))?;

    context
        .register_global_property(js_string!("input"), value, Attribute::all())
        .expect("input property shouldn't exist");

    Ok(())
}

fn restore_input_prototypes(context: &mut Context) -> Result<(), RunComponentError> {
    context
        .eval(Source::from_bytes(
            r#"
            function restorePrototypes(obj) {
                if (!obj || typeof obj !== 'object') return;

                if (Array.isArray(obj)) {
                    Object.setPrototypeOf(obj, Array.prototype);
                } else {
                    Object.setPrototypeOf(obj, Object.prototype);
                }

                for (const key in obj) {
                    restorePrototypes(obj[key]);
                }
            }

            restorePrototypes(input);
            "#,
        ))
        .map_err(|e| {
            RunComponentError::Other(format!("Failed to restore input object prototypes.\n{}", e))
        })?;
    Ok(())
}

fn run_component_scripts(
    script_files: &[String],
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    context: &mut Context,
) -> Result<JsValue, RunComponentError> {
    let mut last_result = None;
    for script_file in script_files.iter() {
        let content = execution_context.files.get_text(script_file)?;

        debug!(
            "Running script \"{}\" with {} bytes",
            script_file,
            content.len()
        );

        last_result = Some(context.eval(Source::from_bytes(&*content)).map_err(|e| {
            RunComponentError::Other(format!("Failed to run script \"{}\"\n{}", script_file, e))
        })?);
    }
    let last_result = last_result.expect("At least one script should be executed");
    Ok(last_result)
}

fn convert_output(
    context: &mut Context,
    last_result: JsValue,
) -> Result<serde_json::Value, RunComponentError> {
    last_result
        .to_json(context)
        .map_err(|e| RunComponentError::Other(format!("Failed to convert output object.\n{}", e)))
}
