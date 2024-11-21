use bytes::Bytes;
use slipway_lib::ComponentHandle;
use tracing::{debug, error, info, trace, warn};
use wasmtime::*;
use wasmtime_wasi::{
    HostOutputStream, ResourceTable, StdoutStream, StreamResult, Subscribe, WasiCtx, WasiView,
};

// https://docs.wasmtime.dev/api/wasmtime/component/bindgen_examples/index.html
// https://component-model.bytecodealliance.org/design/wit.html
// https://component-model.bytecodealliance.org/language-support/rust.html
// https://lib.rs/crates/wasmtime-wasi
wasmtime::component::bindgen!({
    path: "../../wit/0.1.0"
});

pub struct SlipwayHost<'a> {
    component_handle: &'a ComponentHandle,
    wasi_ctx: WasiCtx,
    wasi_table: ResourceTable,
}

impl<'a> SlipwayHost<'a> {
    pub fn new(component_handle: &'a ComponentHandle, wasi_ctx: WasiCtx) -> Self {
        Self {
            component_handle,
            wasi_ctx,
            wasi_table: ResourceTable::default(),
        }
    }
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
pub enum OutputObserverType {
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

pub struct OutputObserverStream {
    component_handle: ComponentHandle,
    observer_type: OutputObserverType,
}

impl OutputObserverStream {
    pub fn new(component_handle: &ComponentHandle, observer_type: OutputObserverType) -> Self {
        Self {
            component_handle: component_handle.clone(),
            observer_type,
        }
    }
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
