use self::slipway_host::{BinResponse, RequestError, RequestOptions, ResolvedFont, TextResponse};
use bytes::Bytes;
use slipway_engine::ComponentExecutionContext;
use tracing::{error, info};
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

impl self::slipway_host::Host for SlipwayHost<'_, '_, '_> {
    fn try_resolve_font(&mut self, font_stack: String) -> Option<ResolvedFont> {
        ::slipway_host::fonts::try_resolve(font_stack).map(|resolved| ResolvedFont {
            family: resolved.family,
            data: resolved.data,
        })
    }

    fn log_trace(&mut self, message: String) {
        ::slipway_host::log::log_trace(message);
    }

    fn log_debug(&mut self, message: String) {
        ::slipway_host::log::log_debug(message);
    }

    fn log_info(&mut self, message: String) {
        ::slipway_host::log::log_info(message);
    }

    fn log_warn(&mut self, message: String) {
        ::slipway_host::log::log_warn(message);
    }

    fn log_error(&mut self, message: String) {
        ::slipway_host::log::log_error(message);
    }

    fn fetch_bin(
        &mut self,
        url: String,
        opts: Option<RequestOptions>,
    ) -> Result<BinResponse, RequestError> {
        ::slipway_host::fetch::fetch(self.execution_context, &url, opts.map(Into::into))
            .map(Into::into)
            .map_err(Into::into)
    }

    fn fetch_text(
        &mut self,
        url: String,
        opts: Option<RequestOptions>,
    ) -> Result<TextResponse, RequestError> {
        ::slipway_host::fetch::fetch(self.execution_context, &url, opts.map(Into::into))
            .map(Into::into)
            .map_err(Into::into)
    }

    fn run(&mut self, handle: String, input: String) -> Result<String, ComponentError> {
        self.fetch_text(
            format!("component://{}", handle),
            Some(RequestOptions {
                body: Some(input.into_bytes()),
                method: None,
                headers: None,
                timeout_ms: None,
            }),
        )
        .map(|v| v.body)
        .map_err(Into::into)
    }

    fn load_bin(&mut self, handle: String, path: String) -> Result<Vec<u8>, ComponentError> {
        self.fetch_bin(format!("component://{}/{}", handle, path), None)
            .map(|v| v.body)
            .map_err(Into::into)
    }

    fn load_text(&mut self, handle: String, path: String) -> Result<String, ComponentError> {
        self.fetch_text(format!("component://{}/{}", handle, path), None)
            .map(|v| v.body)
            .map_err(Into::into)
    }
}

impl From<::slipway_host::fetch::RequestError> for RequestError {
    fn from(e: ::slipway_host::fetch::RequestError) -> Self {
        RequestError {
            message: e.message,
            response: e.response.map(|r| BinResponse {
                status: r.status,
                headers: r.headers,
                body: r.body,
            }),
        }
    }
}

impl From<RequestError> for ComponentError {
    fn from(e: RequestError) -> Self {
        ComponentError { message: e.message }
    }
}

impl From<RequestOptions> for ::slipway_host::fetch::RequestOptions {
    fn from(opts: RequestOptions) -> Self {
        ::slipway_host::fetch::RequestOptions {
            headers: opts.headers,
            method: opts.method,
            body: opts.body,
            timeout_ms: opts.timeout_ms,
        }
    }
}

impl From<::slipway_host::fetch::Response> for BinResponse {
    fn from(r: ::slipway_host::fetch::Response) -> Self {
        BinResponse {
            status: r.status,
            headers: r.headers,
            body: r.body,
        }
    }
}

impl From<::slipway_host::fetch::Response> for TextResponse {
    fn from(r: ::slipway_host::fetch::Response) -> Self {
        TextResponse {
            status: r.status,
            headers: r.headers,
            body: String::from_utf8_lossy(&r.body).into_owned(),
        }
    }
}

impl slipway::component::types::Host for SlipwayHost<'_, '_, '_> {}

impl From<::slipway_host::ComponentError> for ComponentError {
    fn from(e: ::slipway_host::ComponentError) -> Self {
        ComponentError { message: e.message }
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
