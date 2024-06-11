use slipway_lib::ComponentExecutionData;
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    sync::WasiCtxBuilder,
};
use wasmtime::*;

use self::errors::WasmExecutionError;

pub(super) mod errors;

pub(super) fn run_component_wasm(
    execution_data: ComponentExecutionData,
) -> Result<serde_json::Value, WasmExecutionError> {
    // Serialize the input JSON to a vector of bytes
    let input_bytes = serde_json::to_vec(&execution_data.input.value)
        .map_err(WasmExecutionError::SerializeInputFailed)?;

    // Create a pipe for stdin and stdout
    let stdin = ReadPipe::from(input_bytes);
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();

    // Create an engine and store
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    // Add WASI to linker
    wasi_common::sync::add_to_linker(&mut linker, |s| s)?;

    // Create a WASI context, including stdin and stdout pipes
    let wasi = WasiCtxBuilder::new()
        .stdin(Box::new(stdin.clone()))
        .stdout(Box::new(stdout.clone()))
        .stderr(Box::new(stderr.clone()))
        .build();

    // Create a store
    let mut store = Store::new(&engine, wasi);

    // Compile the module
    let module = Module::new(&engine, &*execution_data.wasm_bytes)?;

    linker.module(&mut store, "", &module)?;

    // Create an instance of the module
    let instance = linker.instantiate(&mut store, &module)?;

    // Get the WASM function
    let wasm_func = instance
        .get_func(&mut store, "step")
        .ok_or(WasmExecutionError::StepCallNotFound())?
        .typed::<(), ()>(&store)
        .map_err(WasmExecutionError::StepCallUnexpectedSignature)?;

    // Call the function
    let call_result = wasm_func.call(&mut store, ());

    // Drop the store so we can read from the stdout pipe
    drop(store);

    // Read the contents of the stderr pipe
    let stderr_contents: Vec<u8> = stderr
        .try_into_inner()
        .expect("sole remaining reference")
        .into_inner();

    // If the stderr pipe is not empty, return an error
    if !stderr_contents.is_empty() {
        let stderr_string = String::from_utf8(stderr_contents)
            .map_err(|_err| anyhow::Error::msg("stderr is not valid UTF-8"))?;

        // If the call result is an error, include it in the error message
        call_result
            .map_err(|e| WasmExecutionError::StepCallFailed(stderr_string.clone(), Some(e)))?;

        // Otherwise, return an error with the stderr contents
        return Err(WasmExecutionError::StepCallFailed(stderr_string, None));
    }

    // If the call result is an error, and there was no stderr, then return a more generic error message.
    call_result.map_err(|e| {
        WasmExecutionError::StepCallFailed("step call returned an error".to_string(), Some(e))
    })?;

    // Read the contents of the stdout pipe
    let stdout_contents: Vec<u8> = stdout
        .try_into_inner()
        .expect("sole remaining reference")
        .into_inner();

    // Deserialize the output JSON
    let output: serde_json::Value = serde_json::from_slice(&stdout_contents)
        .map_err(WasmExecutionError::DeserializeOutputFailed)?;

    Ok(output)
}
