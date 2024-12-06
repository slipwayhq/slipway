use std::{sync::Arc, time::Instant};

use host::{OutputObserverStream, OutputObserverType, SlipwayComponent, SlipwayHost};
use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;

mod host;

pub fn run_component_wasm(
    input: &serde_json::Value,
    wasm_bytes: Arc<Vec<u8>>,
    execution_context: &ComponentExecutionContext,
) -> Result<RunComponentResult, RunComponentError> {
    let prepare_input_start = Instant::now();

    let handle = execution_context.component_handle();

    // Serialize the input JSON to a vector of bytes
    let input_string = serde_json::to_string(input)
        .map_err(|source| RunComponentError::SerializeInputFailed { source })?;

    let prepare_input_duration = prepare_input_start.elapsed();
    let prepare_component_start = Instant::now();

    // Create an engine and store
    let engine = Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);

    // Add WASI to linker
    SlipwayComponent::add_to_linker(&mut linker, |state: &mut SlipwayHost| state)?;
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    // Create a WASI context, including stdin and stdout pipes
    let stdout = OutputObserverStream::new(handle, OutputObserverType::Stdout);
    let stderr = OutputObserverStream::new(handle, OutputObserverType::Stderr);
    let wasi_ctx = WasiCtxBuilder::new().stdout(stdout).stderr(stderr).build();

    // Create a store
    let mut store = Store::new(
        &engine,
        SlipwayHost::new(handle, execution_context, wasi_ctx),
    );

    // Create the component from raw bytes.
    let component = wasmtime::component::Component::new(&engine, &*wasm_bytes)?;

    // Create the SlipwayComponent instance.
    let slipway_component = SlipwayComponent::instantiate(&mut store, &component, &linker)?;

    let prepare_component_duration = prepare_component_start.elapsed();

    // Call the function
    let call_start = Instant::now();
    let call_result = slipway_component.call_run(&mut store, &input_string);
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();

    // Process the result.
    match call_result {
        Err(e) => Err(RunComponentError::RunCallFailed { source: e }),
        Ok(r) => match r {
            // The WASM component returned an error from it's `run` function.
            Err(error) => Err(RunComponentError::RunCallReturnedError {
                message: error.message,
            }),
            Ok(json_string) => {
                // Deserialize the output JSON
                let output = serde_json::from_str(&json_string)
                    .map_err(|source| RunComponentError::DeserializeOutputFailed { source })?;

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
        },
    }
}
