use bytes::Bytes;
use slipway_engine::ComponentExecutionContext;
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
    path: "../../wit/latest"
});

pub struct SlipwayHost<'call, 'rig, 'runners> {
    execution_context: &'call ComponentExecutionContext<'call, 'rig, 'runners>,
    wasi_ctx: WasiCtx,
    wasi_table: ResourceTable,
}

impl<'call, 'rig, 'runners> SlipwayHost<'call, 'rig, 'runners> {
    pub fn new(
        execution_data: &'call ComponentExecutionContext<'call, 'rig, 'runners>,
        wasi_ctx: WasiCtx,
    ) -> Self {
        Self {
            execution_context: execution_data,
            wasi_ctx,
            wasi_table: ResourceTable::default(),
        }
    }
}

impl WasiView for SlipwayHost<'_, '_, '_> {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.wasi_table
    }
}

impl font::Host for SlipwayHost<'_, '_, '_> {
    fn try_resolve(&mut self, font_stack: String) -> Option<font::ResolvedFont> {
        slipway_host::fonts::try_resolve(font_stack).map(|resolved| font::ResolvedFont {
            family: resolved.family,
            data: resolved.data,
        })
    }
}

impl callout::Host for SlipwayHost<'_, '_, '_> {
    fn run(&mut self, handle: String, input: String) -> Result<String, ComponentError> {
        slipway_host::run::run_component_callout_for_host(self.execution_context, &handle, &input)
            .map_err(|e| e.into())
    }
}

impl slipway::component::types::Host for SlipwayHost<'_, '_, '_> {}

impl From<slipway_host::ComponentError> for ComponentError {
    fn from(e: slipway_host::ComponentError) -> Self {
        ComponentError { message: e.message }
    }
}

impl log::Host for SlipwayHost<'_, '_, '_> {
    fn trace(&mut self, message: String) {
        trace!(message);
    }

    fn debug(&mut self, message: String) {
        debug!(message);
    }

    fn info(&mut self, message: String) {
        info!(message);
    }

    fn warn(&mut self, message: String) {
        warn!(message);
    }

    fn error(&mut self, message: String) {
        error!(message);
    }
}

#[derive(Copy, Clone, Debug)]
pub enum OutputObserverType {
    Stdout,
    Stderr,
}
struct OutputObserver {
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
            self.log_line(&self.buffer);
        }
    }
}

impl HostOutputStream for OutputObserver {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete lines
        while let Some(pos) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=pos).collect();
            self.log_line(line.trim());
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
    fn log_line(&self, message: &str) {
        match self.observer_type {
            OutputObserverType::Stdout => {
                info!(message);
            }
            OutputObserverType::Stderr => {
                error!(message);
            }
        }
    }
}

pub struct OutputObserverStream {
    observer_type: OutputObserverType,
}

impl OutputObserverStream {
    pub fn new(observer_type: OutputObserverType) -> Self {
        Self { observer_type }
    }
}

impl StdoutStream for OutputObserverStream {
    fn stream(&self) -> Box<dyn wasmtime_wasi::HostOutputStream> {
        Box::new(OutputObserver {
            buffer: String::new(),
            observer_type: self.observer_type,
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}
