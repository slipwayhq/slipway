use std::time::Instant;

use slipway_host::{OutputObserverStream, OutputObserverType, SlipwayComponent, SlipwayHost};
use slipway_lib::{ComponentExecutionData, ComponentHandle, RunMetadata};
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;

use self::errors::WasmExecutionError;

mod slipway_host;

pub(super) mod errors;

pub(super) struct RunComponentWasmResult {
    pub output: serde_json::Value,
    pub metadata: RunMetadata,
}

pub(super) fn run_component_wasm(
    execution_data: ComponentExecutionData,
    handle: &ComponentHandle,
) -> Result<RunComponentWasmResult, WasmExecutionError> {
    let prepare_input_start = Instant::now();

    // Serialize the input JSON to a vector of bytes
    let input_string = serde_json::to_string(&execution_data.input.value)
        .map_err(|source| WasmExecutionError::SerializeInputFailed { source })?;

    let prepare_input_duration = prepare_input_start.elapsed();
    let prepare_component_start = Instant::now();

    // Create an engine and store
    let engine = Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);

    // Add WASI to linker
    SlipwayComponent::add_to_linker(&mut linker, |state: &mut SlipwayHost| state)?;
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    let stdout = OutputObserverStream::new(handle, OutputObserverType::Stdout);
    let stderr = OutputObserverStream::new(handle, OutputObserverType::Stderr);

    // Create a WASI context, including stdin and stdout pipes
    let wasi_ctx = WasiCtxBuilder::new()
        // .inherit_env()
        // .expect("temp")
        // .stdin(Box::new(WritePipe::new_in_memory()))
        .stdout(stdout)
        .stderr(stderr)
        .build();

    // Create a store
    let mut store = Store::new(&engine, SlipwayHost::new(handle, wasi_ctx));

    let component = wasmtime::component::Component::new(&engine, &*execution_data.wasm_bytes)?;

    let instance = SlipwayComponent::instantiate(&mut store, &component, &linker)?;

    let prepare_component_duration = prepare_component_start.elapsed();

    // Call the function
    let call_start = Instant::now();
    let call_result = instance.call_run(&mut store, &input_string);
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();

    // Process the result.
    match call_result {
        Err(e) => Err(WasmExecutionError::RunCallFailed { source: Some(e) }),
        Ok(r) => match r {
            // The WASM component returned an error from it's `run` function.
            Err(error) => Err(WasmExecutionError::RunCallReturnedError { error }),
            Ok(json_string) => {
                // Deserialize the output JSON
                let output = serde_json::from_str(&json_string)
                    .map_err(|source| WasmExecutionError::DeserializeOutputFailed { source })?;

                let process_output_duration = process_output_start.elapsed();

                Ok(RunComponentWasmResult {
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
