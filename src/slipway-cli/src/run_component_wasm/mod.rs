use slipway_lib::{ComponentExecutionData, ComponentHandle};
use tracing::info;
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    sync::WasiCtxBuilder,
};
use wasmtime::*;

use self::errors::WasmExecutionError;

pub(super) mod errors;

pub(super) fn run_component_wasm(
    execution_data: ComponentExecutionData,
    handle: &ComponentHandle,
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

    // Log all output lines until we have a JSON object at the start of a line.
    let json_index = write_until_bracket(&stdout_contents, handle);

    // Deserialize the output JSON
    let output: serde_json::Value = serde_json::from_slice(&stdout_contents[json_index..])
        .map_err(WasmExecutionError::DeserializeOutputFailed)?;

    Ok(output)
}

/// Write all lines from the input until a JSON object is found at the start of a line.
/// Returns the index of the first byte of the JSON object.
fn write_until_bracket(input: &[u8], component_handle: &ComponentHandle) -> usize {
    let mut buffer = Vec::new();
    let mut prev_byte = b'\n';
    let mut json_index = 0;
    for &byte in input.iter() {
        if prev_byte == b'\n' && byte == b'{' {
            break;
        }
        buffer.push(byte);
        if byte == b'\n' {
            if let Ok(line) = String::from_utf8(buffer.clone()) {
                info!(
                    component = component_handle.to_string(),
                    "{}",
                    line.trim_end()
                );
            }
            buffer.clear();
        }
        prev_byte = byte;
        json_index += 1;
    }

    // Log any remaining buffered data
    if !buffer.is_empty() {
        if let Ok(line) = String::from_utf8(buffer) {
            info!("{}", line.trim_end());
        }
    }

    json_index
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn write_until_bracket_should_write_lines_until_it_finds_a_bracket_after_a_newline() {
        let input = b"line 1\nline 2\nline 3\n{ \"key\": \"value\" }";
        let json_index = write_until_bracket(input, &ComponentHandle::from_str("test").unwrap());
        assert_eq!(json_index, 21);
    }
}
