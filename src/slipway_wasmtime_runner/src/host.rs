use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use self::slipway_host::{BinResponse, RequestError, RequestOptions, ResolvedFont, TextResponse};
use bytes::Bytes;
use slipway_engine::ComponentExecutionContext;
use tracing::{error, info};
use wasmtime::*;
use wasmtime_wasi::{
    IoView, OutputStream, Pollable, ResourceTable, StdoutStream, StreamResult, WasiCtx, WasiView,
};

// https://docs.wasmtime.dev/api/wasmtime/component/bindgen_examples/index.html
// https://component-model.bytecodealliance.org/design/wit.html
// https://component-model.bytecodealliance.org/language-support/rust.html
// https://lib.rs/crates/wasmtime-wasi
wasmtime::component::bindgen!({
    path: "../../wit/latest",
    async: true
});

pub struct SlipwayHost<'call, 'rig, 'runners> {
    execution_context: &'call ComponentExecutionContext<'call, 'rig, 'runners>,
    wasi_ctx: WasiCtx,
    wasi_table: ResourceTable,
}

impl<'call, 'rig, 'runners> SlipwayHost<'call, 'rig, 'runners> {
    pub fn new(
        execution_context: &'call ComponentExecutionContext<'call, 'rig, 'runners>,
        wasi_ctx: WasiCtx,
    ) -> Self {
        Self {
            execution_context,
            wasi_ctx,
            wasi_table: ResourceTable::default(),
        }
    }
}

impl IoView for SlipwayHost<'_, '_, '_> {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.wasi_table
    }
}

impl WasiView for SlipwayHost<'_, '_, '_> {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

/// A wrapper that unsafely asserts its inner future is Send.
/// Wasmtime requires that host functions are Send:
/// https://docs.rs/wasmtime/latest/wasmtime/struct.Func.html#why-send--sync--static
/// Unfortunately our host functions can call into Boa, which cannot be Send:
/// https://github.com/boa-dev/boa/discussions/4001
/// However, wasmtime itself doesn't actually spawn any threads, so pretending
/// our futures are Send should be safe as long as we run in a single threaded async
/// runtime.
/// https://github.com/bytecodealliance/wasmtime/issues/5936
struct AssertSend<F: ?Sized>(F);
unsafe impl<F: ?Sized> Send for AssertSend<F> {}
impl<F: Future + ?Sized> Future for AssertSend<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: We’re simply forwarding the poll; this is only safe
        // if we can guarantee that the future is never used concurrently
        // on multiple threads.
        unsafe { self.map_unchecked_mut(|s| &mut s.0) }.poll(cx)
    }
}

impl self::slipway_host::Host for SlipwayHost<'_, '_, '_> {
    fn font(
        &mut self,
        font_stack: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = Option<ResolvedFont>> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::fonts::font(self.execution_context, font_stack)
                .await
                .map(|resolved| ResolvedFont {
                    family: resolved.family,
                    data: resolved.data,
                })
        })
    }

    fn log_trace(
        &mut self,
        message: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::log::log_trace(message);
        })
    }

    fn log_debug(
        &mut self,
        message: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::log::log_debug(message);
        })
    }

    fn log_info(
        &mut self,
        message: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::log::log_info(message);
        })
    }

    fn log_warn(
        &mut self,
        message: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::log::log_warn(message);
        })
    }

    fn log_error(
        &mut self,
        message: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::log::log_error(message);
        })
    }

    fn fetch_bin(
        &mut self,
        url: wasmtime::component::__internal::String,
        options: Option<RequestOptions>,
    ) -> impl ::core::future::Future<Output = Result<BinResponse, RequestError>> + ::core::marker::Send
    {
        Box::pin(AssertSend(async move {
            ::slipway_host::fetch::fetch_bin(self.execution_context, &url, options.map(Into::into))
                .await
                .map(Into::into)
                .map_err(Into::into)
        }))
    }

    fn fetch_text(
        &mut self,
        url: wasmtime::component::__internal::String,
        options: Option<RequestOptions>,
    ) -> impl ::core::future::Future<Output = Result<TextResponse, RequestError>> + ::core::marker::Send
    {
        Box::pin(AssertSend(async move {
            ::slipway_host::fetch::fetch_text(self.execution_context, &url, options.map(Into::into))
                .await
                .map(Into::into)
                .map_err(Into::into)
        }))
    }

    fn run(
        &mut self,
        handle: wasmtime::component::__internal::String,
        input: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<
        Output = Result<wasmtime::component::__internal::String, ComponentError>,
    > + ::core::marker::Send {
        Box::pin(AssertSend(async {
            ::slipway_host::fetch::run_string(self.execution_context, handle, input)
                .await
                .map_err(Into::into)
        }))
    }

    fn load_bin(
        &mut self,
        handle: wasmtime::component::__internal::String,
        path: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<
        Output = Result<wasmtime::component::__internal::Vec<u8>, ComponentError>,
    > + ::core::marker::Send {
        Box::pin(AssertSend(async {
            ::slipway_host::fetch::load_bin(self.execution_context, handle, path)
                .await
                .map_err(Into::into)
        }))
    }

    fn load_text(
        &mut self,
        handle: wasmtime::component::__internal::String,
        path: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<
        Output = Result<wasmtime::component::__internal::String, ComponentError>,
    > + ::core::marker::Send {
        Box::pin(AssertSend(async {
            ::slipway_host::fetch::load_text(self.execution_context, handle, path)
                .await
                .map_err(Into::into)
        }))
    }

    fn env(
        &mut self,
        key: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<Output = Option<wasmtime::component::__internal::String>>
    + ::core::marker::Send {
        Box::pin(async move { ::slipway_host::fetch::env(self.execution_context, &key) })
    }

    fn encode_bin(
        &mut self,
        bin: wasmtime::component::__internal::Vec<u8>,
    ) -> impl ::core::future::Future<Output = wasmtime::component::__internal::String>
    + ::core::marker::Send {
        Box::pin(async { ::slipway_host::bin::encode_bin(self.execution_context, bin) })
    }

    fn decode_bin(
        &mut self,
        text: wasmtime::component::__internal::String,
    ) -> impl ::core::future::Future<
        Output = Result<wasmtime::component::__internal::Vec<u8>, ComponentError>,
    > + ::core::marker::Send {
        Box::pin(async {
            ::slipway_host::bin::decode_bin(self.execution_context, text).map_err(Into::into)
        })
    }
}

impl From<::slipway_host::fetch::RequestError> for RequestError {
    fn from(e: ::slipway_host::fetch::RequestError) -> Self {
        RequestError {
            message: e.message,
            inner: e.inner,
            response: e.response.map(|r| TextResponse {
                status_code: r.status_code,
                headers: r.headers,
                body: r.body,
            }),
        }
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

impl From<::slipway_host::fetch::BinResponse> for BinResponse {
    fn from(r: ::slipway_host::fetch::BinResponse) -> Self {
        BinResponse {
            status_code: r.status_code,
            headers: r.headers,
            body: r.body,
        }
    }
}

impl From<::slipway_host::fetch::TextResponse> for TextResponse {
    fn from(r: ::slipway_host::fetch::TextResponse) -> Self {
        TextResponse {
            status_code: r.status_code,
            headers: r.headers,
            body: r.body,
        }
    }
}

impl From<::slipway_host::ComponentError> for ComponentError {
    fn from(e: ::slipway_host::ComponentError) -> Self {
        ComponentError {
            message: e.message,
            inner: e.inner,
        }
    }
}

impl slipway::component::types::Host for SlipwayHost<'_, '_, '_> {}

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
impl Pollable for OutputObserver {
    async fn ready(&mut self) {}
}

impl Drop for OutputObserver {
    fn drop(&mut self) {
        if !self.buffer.is_empty() {
            self.log_line(&self.buffer);
        }
    }
}

impl OutputStream for OutputObserver {
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
    fn stream(&self) -> Box<dyn OutputStream> {
        Box::new(OutputObserver {
            buffer: String::new(),
            observer_type: self.observer_type,
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}
