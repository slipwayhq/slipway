use std::{sync::Arc, time::Instant};

use crate::host::{OutputObserverStream, OutputObserverType, Slipway, SlipwayHost};
use slipway_engine::{
    ComponentExecutionContext, RunComponentError, RunComponentResult, RunMetadata,
};
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;

pub enum WasmData {
    Wasm(Arc<Vec<u8>>),
    Aot(Vec<u8>),
}

pub async fn run_component_wasm(
    input: &serde_json::Value,
    wasm_data: WasmData,
    engine: &Engine,
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
) -> Result<RunComponentResult, RunComponentError> {
    let prepare_input_start = Instant::now();

    // Serialize the input JSON to a vector of bytes
    let input_string = serde_json::to_string(input)
        .map_err(|source| RunComponentError::SerializeInputFailed { source })?;

    let prepare_input_duration = prepare_input_start.elapsed();
    let prepare_component_start = Instant::now();

    // Create a linker.
    let mut linker = wasmtime::component::Linker::new(engine);

    // Add WASI to linker
    Slipway::add_to_linker(&mut linker, |state: &mut SlipwayHost| state)?;
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // Create a WASI context, including stdin and stdout pipes
    let stdout = OutputObserverStream::new(OutputObserverType::Stdout);
    let stderr = OutputObserverStream::new(OutputObserverType::Stderr);
    let wasi_ctx = WasiCtxBuilder::new()
        .stdout(stdout)
        .stderr(stderr)
        .env("TZ", &execution_context.rig_session_options.timezone)
        .build();

    // Create a store
    let mut store = Store::new(engine, SlipwayHost::new(execution_context, wasi_ctx));

    // Create the component from raw bytes.
    let component = match wasm_data {
        WasmData::Wasm(wasm_bytes) => wasmtime::component::Component::new(engine, &*wasm_bytes)?,
        WasmData::Aot(aot_bytes) => unsafe {
            wasmtime::component::Component::deserialize(engine, &aot_bytes)?
        },
    };

    // Create the SlipwayComponent instance.
    let slipway_component = Slipway::instantiate_async(&mut store, &component, &linker).await?;

    let prepare_component_duration = prepare_component_start.elapsed();

    // Call the function
    let call_start = Instant::now();
    let call_result = slipway_component.call_run(&mut store, &input_string).await;
    let call_duration = call_start.elapsed();

    let process_output_start = Instant::now();

    // Process the result.
    match call_result {
        Err(e) => Err(RunComponentError::RunCallFailed { source: e }),
        Ok(r) => match r {
            // The WASM component returned an error from it's `run` function.
            Err(error) => Err(RunComponentError::RunCallReturnedError {
                message: error.message,
                inner: error.inner,
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
