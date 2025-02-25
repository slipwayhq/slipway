use std::{sync::Arc, time::Instant};

use deno_core::{
    serde_v8,
    v8::{self},
    Extension, JsRuntime, RuntimeOptions,
};
use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};

use crate::{deno_ops::DECLS, DenoComponentDefinition, DENO_COMPONENT_DEFINITION_FILE_NAME};

pub(super) async fn run_component_javascript(
    input: &serde_json::Value,
    deno_definition: Arc<DenoComponentDefinition>,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunComponentError> {
    if deno_definition.scripts.is_empty() {
        return Err(RunComponentError::Other(format!(
            "No scripts specified in definition file \"{}\"",
            DENO_COMPONENT_DEFINITION_FILE_NAME
        )));
    }

    let prepare_component_start = Instant::now();
    let mut runtime = create_js_runtime();
    prepare_environment(&mut runtime)?;
    let prepare_component_duration = prepare_component_start.elapsed();

    let prepare_input_start = Instant::now();
    set_input(&mut runtime, input)?;
    restore_input_prototypes(&mut runtime)?;
    let prepare_input_duration = prepare_input_start.elapsed();

    let call_start = Instant::now();
    let last_result =
        run_component_scripts(&deno_definition.scripts, execution_context, &mut runtime).await?;
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();
    let value = convert_output(runtime, last_result);
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
        Err(error) => Err(RunComponentError::GenericError(error.into())),
    }
}

fn create_js_runtime() -> JsRuntime {
    let ext = Extension {
        name: "slipway_ext",
        ops: std::borrow::Cow::Borrowed(&DECLS),
        ..Default::default()
    };
    JsRuntime::new(RuntimeOptions {
        extensions: vec![ext],
        ..RuntimeOptions::default()
    })
}

fn prepare_environment(runtime: &mut JsRuntime) -> Result<(), RunComponentError> {
    runtime
        .execute_script(
            "[anon]",
            r#"
            ((globalThis) => {
                const core = Deno.core;

                function argsToMessage(...args) {
                    return args.map((arg) => JSON.stringify(arg)).join(" ");
                }

                globalThis.console = {
                    trace: (...args) => {
                        core.ops.op_trace(`${argsToMessage(...args)}`);
                    },
                    debug: (...args) => {
                        core.ops.op_debug(`${argsToMessage(...args)}`);
                    },
                    log: (...args) => {
                        core.ops.op_info(`${argsToMessage(...args)}`);
                    },
                    warn: (...args) => {
                        core.ops.op_warn(`${argsToMessage(...args)}`);
                    },
                    error: (...args) => {
                        core.ops.op_error(`${argsToMessage(...args)}`);
                    },
                }

                globalThis.global = {};
                globalThis.setTimeout = () => { };
                globalThis.clearTimeout = () => { };
            })(globalThis);
            "#,
        )
        .map_err(RunComponentError::GenericError)?;
    Ok(())
}

fn set_input(runtime: &mut JsRuntime, input: &serde_json::Value) -> Result<(), RunComponentError> {
    let scope = &mut runtime.handle_scope();
    let context = scope.get_current_context();
    let v8_input =
        serde_v8::to_v8(scope, input).map_err(|e| RunComponentError::GenericError(e.into()))?;
    let global = context.global(scope);
    let input_key = v8::String::new(scope, "input").expect("Input key should be valid");
    global.set(scope, input_key.into(), v8_input);
    Ok(())
}

fn restore_input_prototypes(runtime: &mut JsRuntime) -> Result<(), RunComponentError> {
    runtime
        .execute_script(
            "[anon]",
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
        )
        .map_err(RunComponentError::GenericError)?;
    Ok(())
}

async fn run_component_scripts(
    script_files: &[String],
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    runtime: &mut JsRuntime,
) -> Result<v8::Global<v8::Value>, RunComponentError> {
    let mut last_result = None;
    for script_file in script_files.iter().cloned() {
        let content = execution_context.files.get_text(&script_file).await?;
        let static_name: &'static str = Box::leak(script_file.into_boxed_str()); // TODO: Remove this leak.
        last_result = Some(
            runtime
                .execute_script(static_name, content.as_ref().clone())
                .map_err(|e| RunComponentError::RunCallFailed { source: e })?,
        );
    }
    let last_result = last_result.expect("At least one script should be executed");
    Ok(last_result)
}

fn convert_output(
    mut runtime: JsRuntime,
    last_result: v8::Global<v8::Value>,
) -> Result<serde_json::Value, serde_v8::Error> {
    let scope = &mut runtime.handle_scope();
    let local = v8::Local::new(scope, last_result);
    serde_v8::from_v8::<serde_json::Value>(scope, local)
}
