use std::time::Instant;

use bytes::Bytes;
use slipway_lib::{ComponentExecutionData, ComponentHandle, RunMetadata};
use tracing::{debug, error, info, trace, warn};
use wasmtime::*;
use wasmtime_wasi::{
    HostOutputStream, ResourceTable, StdoutStream, StreamResult, Subscribe, WasiCtx,
    WasiCtxBuilder, WasiView,
};

use super::{errors::WasmExecutionError, RunComponentWasmResult};

// https://docs.wasmtime.dev/api/wasmtime/component/bindgen_examples/index.html
// https://component-model.bytecodealliance.org/design/wit.html
// https://component-model.bytecodealliance.org/language-support/rust.html
// https://lib.rs/crates/wasmtime-wasi
wasmtime::component::bindgen!({
    path: "../../wit/0.1.0"
});

struct SlipwayHost<'a> {
    component_handle: &'a ComponentHandle,
    wasi_ctx: WasiCtx,
    wasi_table: ResourceTable,
}

impl<'a> WasiView for SlipwayHost<'a> {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.wasi_table
    }
}

impl<'a> font::Host for SlipwayHost<'a> {
    fn resolve(&mut self, _names: Vec<String>) -> Vec<u8> {
        unimplemented!("font resolve not implemented");
    }

    fn try_resolve(&mut self, _names: Vec<String>) -> Option<Vec<u8>> {
        unimplemented!("font try-resolve not implemented");
    }
}

impl<'a> component::Host for SlipwayHost<'a> {
    fn run(&mut self, _handle: String, _input: String) -> Result<String, String> {
        unimplemented!("component run not implemented");
    }
}

impl<'a> log::Host for SlipwayHost<'a> {
    fn trace(&mut self, message: String) {
        trace!(component = self.component_handle.to_string(), message);
    }

    fn debug(&mut self, message: String) {
        debug!(component = self.component_handle.to_string(), message);
    }

    fn info(&mut self, message: String) {
        info!(component = self.component_handle.to_string(), message);
    }

    fn warn(&mut self, message: String) {
        warn!(component = self.component_handle.to_string(), message);
    }

    fn error(&mut self, message: String) {
        error!(component = self.component_handle.to_string(), message);
    }
}

#[derive(Copy, Clone, Debug)]
enum OutputObserverType {
    Stdout,
    Stderr,
}
struct OutputObserver {
    component_handle: ComponentHandle,
    buffer: String,
    observer_type: OutputObserverType,
}

#[async_trait::async_trait]
impl Subscribe for OutputObserver {
    async fn ready(&mut self) {}
}

impl Drop for OutputObserver {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            self.log_line(self.buffer.clone());
        }
    }
}

impl HostOutputStream for OutputObserver {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete lines
        while let Some(pos) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=pos).collect();
            self.log_line(line);
        }

        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        // This stream is always ready for writing.
        Ok(usize::MAX)
    }
}

impl OutputObserver {
    fn log_line(&self, line: String) {
        match self.observer_type {
            OutputObserverType::Stdout => {
                println!("info: {}", line);
                info!(component = self.component_handle.to_string(), line);
            }
            OutputObserverType::Stderr => {
                println!("stderr: {}", line);
                error!(component = self.component_handle.to_string(), line);
            }
        }
    }
}

struct OutputObserverStream {
    component_handle: ComponentHandle,
    observer_type: OutputObserverType,
}

impl StdoutStream for OutputObserverStream {
    fn stream(&self) -> Box<dyn wasmtime_wasi::HostOutputStream> {
        Box::new(OutputObserver {
            component_handle: self.component_handle.clone(),
            buffer: String::new(),
            observer_type: self.observer_type,
        })
    }

    fn isatty(&self) -> bool {
        false
    }
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

    let stdout = OutputObserverStream {
        component_handle: handle.clone(),
        observer_type: OutputObserverType::Stdout,
    };
    let stderr = OutputObserverStream {
        component_handle: handle.clone(),
        observer_type: OutputObserverType::Stderr,
    };

    // Create an engine and store
    let engine = Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);

    // Add WASI to linker
    SlipwayComponent::add_to_linker(&mut linker, |state: &mut SlipwayHost| state)?;
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    // Create a WASI context, including stdin and stdout pipes
    let wasi_ctx = WasiCtxBuilder::new()
        // .inherit_env()
        // .expect("temp")
        // .stdin(Box::new(WritePipe::new_in_memory()))
        .stdout(stdout)
        .stderr(stderr)
        .build();

    // Create a store
    let mut store = Store::new(
        &engine,
        SlipwayHost {
            component_handle: handle,
            wasi_ctx,
            wasi_table: ResourceTable::new(),
        },
    );

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
        Err(e) => Err(WasmExecutionError::RunCallFailed {
            message: "component step call failed".to_string(),
            source: Some(e),
        }),
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
