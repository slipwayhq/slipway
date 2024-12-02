use std::{rc::Rc, str::FromStr, sync::Arc};

use bytes::Bytes;
use slipway_engine::{
    get_component_execution_data, ComponentExecutionContext, ComponentHandle, ComponentInput,
    JsonMetadata,
};
use slipway_host::run::{run_component, run_component_callout};
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

pub struct SlipwayHost<'rig, 'step> {
    component_handle: &'rig ComponentHandle,
    execution_context: &'step ComponentExecutionContext<'rig>,
    wasi_ctx: WasiCtx,
    wasi_table: ResourceTable,
}

impl<'rig, 'step> SlipwayHost<'rig, 'step> {
    pub fn new(
        component_handle: &'rig ComponentHandle,
        execution_data: &'step ComponentExecutionContext<'rig>,
        wasi_ctx: WasiCtx,
    ) -> Self {
        Self {
            component_handle,
            execution_context: execution_data,
            wasi_ctx,
            wasi_table: ResourceTable::default(),
        }
    }
}

impl<'rig, 'step> WasiView for SlipwayHost<'rig, 'step> {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.wasi_table
    }
}

impl<'rig, 'step> font::Host for SlipwayHost<'rig, 'step> {
    fn try_resolve(&mut self, font_stack: String) -> Option<font::ResolvedFont> {
        slipway_host::fonts::try_resolve(font_stack).map(|resolved| font::ResolvedFont {
            family: resolved.family,
            data: resolved.data,
        })
    }
}

impl<'rig, 'step> component::Host for SlipwayHost<'rig, 'step> {
    fn run(&mut self, handle: String, input: String) -> Result<String, String> {
        // TODO: Hide all this implementation detail.
        let handle = ComponentHandle::from_str(&handle).expect("HMM");

        let component_reference = self
            .execution_context
            .callout_context
            .get_component_reference_for_handle(&handle);

        let component_cache = self.execution_context.callout_context.component_cache;

        let permission_chain = Arc::clone(&self.execution_context.permission_chain);

        // There are no outer callouts if we're already in a callout.
        let outer_callouts = None;

        let component_runners = self.execution_context.component_runners;

        let input = serde_json::from_str(&input).expect("HMM");
        let json_metadata = JsonMetadata::from_value(&input);

        let input = Rc::new(ComponentInput {
            value: input,
            json_metadata,
        });

        let execution_data = get_component_execution_data(
            component_reference,
            component_cache,
            component_runners,
            permission_chain,
            outer_callouts,
            input,
        )
        .expect("HMM");

        let result = run_component_callout::<anyhow::Error>(&handle, &execution_data).expect("HMM");

        Ok(serde_json::to_string(&result.output).expect("HMM"))
    }
}

impl<'rig, 'step> log::Host for SlipwayHost<'rig, 'step> {
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
                info!(component = self.component_handle.to_string(), line);
            }
            OutputObserverType::Stderr => {
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
