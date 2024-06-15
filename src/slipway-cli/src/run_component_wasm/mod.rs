use slipway_lib::{ComponentExecutionData, ComponentHandle};
use tracing::{debug, error, info, trace, warn};
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

    // Read the contents of the stdout pipe
    let stdout_contents: Vec<u8> = stdout
        .try_into_inner()
        .expect("sole remaining reference")
        .into_inner();

    // Log all output lines until we have a JSON object at the start of a line.
    let json_buffer = log_until_json(&stdout_contents, handle);

    // Read the contents of the stderr pipe.
    let stderr_contents: Vec<u8> = stderr
        .try_into_inner()
        .expect("sole remaining reference")
        .into_inner();

    // If there was an error returned, store it.
    let error_result = call_result.err();

    // If the stderr pipe is not empty, or an error was returned, we return an error.
    if error_result.is_some() || !stderr_contents.is_empty() {
        let stderr_string = String::from_utf8(stderr_contents)
            .map_err(|_err| WasmExecutionError::Other("stderr is not valid UTF-8".to_string()))?
            .trim()
            .to_string();

        return Err(WasmExecutionError::StepCallFailed(
            match stderr_string.is_empty() {
                true => "component step call failed".to_string(),
                false => stderr_string,
            },
            error_result,
        ));
    }

    // Deserialize the output JSON
    let output =
        serde_json::from_slice(json_buffer).map_err(WasmExecutionError::DeserializeOutputFailed)?;

    Ok(output)
}

/// Write all lines from the input until a JSON object is found at the start of a line.
/// Returns the slice from the start of the JSON object to the end of the buffer.
fn log_until_json<'buffer>(
    buffer: &'buffer [u8],
    component_handle: &ComponentHandle,
) -> &'buffer [u8] {
    let mut line_buffer = Vec::new();
    let mut prev_byte = b'\n';
    let mut json_index = 0;

    for &byte in buffer.iter() {
        // If we find a curly bracket at the start of a line,
        // then we assume the rest of the buffer is the JSON
        // response.
        if prev_byte == b'\n' && byte == b'{' {
            break;
        }

        // If we're at the end of the line, log the current line buffer
        // and clear it.
        if byte == b'\n' {
            log_component_line(line_buffer.clone(), component_handle);
            line_buffer.clear();
        } else {
            // Otherwise append to the line buffer.
            line_buffer.push(byte);
        }

        prev_byte = byte;
        json_index += 1;
    }

    // Log any remaining data in the line buffer.
    if !line_buffer.is_empty() {
        log_component_line(line_buffer, component_handle);
    }

    // Otherwise return the index of the JSON response.
    &buffer[json_index..]
}

/// Log the line using the appropriate macro.
/// Errors here are simply logged, but do not cause the component
/// to error. The principle here is that simply writing to stdout
/// should not cause the component to fail.
fn log_component_line(buffer: Vec<u8>, component_handle: &ComponentHandle) {
    if let Ok(line) = String::from_utf8(buffer) {
        if let Some(s) = line.strip_prefix("error:") {
            let s = s.trim();
            error!(component = component_handle.to_string(), s);
        } else if let Some(s) = line.strip_prefix("warning:") {
            let s = s.trim();
            warn!(component = component_handle.to_string(), s);
        } else if let Some(s) = line.strip_prefix("warn:") {
            let s = s.trim();
            warn!(component = component_handle.to_string(), s);
        } else if let Some(s) = line.strip_prefix("info:") {
            let s = s.trim();
            info!(component = component_handle.to_string(), s);
        } else if let Some(s) = line.strip_prefix("debug:") {
            let s = s.trim();
            debug!(component = component_handle.to_string(), s);
        } else if let Some(s) = line.strip_prefix("trace:") {
            let s = s.trim();
            trace!(component = component_handle.to_string(), s);
        } else {
            let s = line.trim();
            info!(component = component_handle.to_string(), s);
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    mod log_until_json {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn it_should_write_lines_until_it_finds_a_bracket_after_a_newline() {
            let input = b"line 1\nline 2\nline 3\n{ \"key\": \"value\" }";
            let json_result = log_until_json(input, &ComponentHandle::from_str("test").unwrap());
            assert_eq!(json_result, b"{ \"key\": \"value\" }");
        }
    }
}
