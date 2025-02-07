mod component;
mod env;
mod http;

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use slipway_engine::{ComponentExecutionContext, ComponentHandle};
use tracing::warn;
use url::Url;

use crate::run::run_component_callout;

#[derive(Debug, Default, Deserialize)]
pub struct RequestOptions {
    pub method: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BinResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestError {
    pub message: String,
    pub inner: Vec<String>,
    pub response: Option<BinResponse>,
}

impl RequestError {
    pub fn for_error(message: String, error: Option<String>) -> RequestError {
        RequestError {
            message,
            inner: match error {
                None => vec![],
                Some(e) => vec![format!("{}", e)],
            },
            response: None,
        }
    }
    pub fn for_inner(message: String, inner: Vec<String>) -> RequestError {
        RequestError {
            message,
            inner,
            response: None,
        }
    }
}

impl From<crate::ComponentError> for RequestError {
    fn from(value: crate::ComponentError) -> Self {
        RequestError::for_inner(value.message, value.inner)
    }
}

impl From<RequestError> for crate::ComponentError {
    fn from(e: RequestError) -> Self {
        match e.response {
            None => crate::ComponentError {
                message: e.message,
                inner: e.inner,
            },
            Some(response) => crate::ComponentError {
                message: e.message,
                inner: std::iter::once(format!("{:?}", response))
                    .chain(e.inner)
                    .collect(),
            },
        }
    }
}

impl From<BinResponse> for TextResponse {
    fn from(r: BinResponse) -> Self {
        TextResponse {
            status_code: r.status_code,
            headers: r.headers,
            body: String::from_utf8_lossy(&r.body).into_owned(),
        }
    }
}

pub fn fetch_bin(
    execution_context: &ComponentExecutionContext,
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    let url = Url::parse(url_str).map_err(|e| {
        RequestError::for_error(
            format!(
                "Failed to parse URL from component {}: {}",
                execution_context.call_chain.component_handle_trail(),
                url_str,
            ),
            Some(format!("{e}")),
        )
    })?;

    let scheme = url.scheme();

    match scheme {
        "https" | "http" => http::fetch_http(execution_context, url, options),
        "component" => component::fetch_component_data(execution_context, &url, options),
        "env" => env::fetch_env(execution_context, &url),
        _ => Err(RequestError::for_error(
            format!(
                "Unsupported URL scheme for URL from component {}: {}",
                execution_context.call_chain.component_handle_trail(),
                url_str
            ),
            None,
        )),
    }
}

pub fn fetch_text(
    execution_context: &ComponentExecutionContext,
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<TextResponse, RequestError> {
    fetch_bin(execution_context, url_str, options)
        .map(Into::into)
        .map_err(Into::into)
}

pub fn run_string(
    execution_context: &ComponentExecutionContext,
    handle: String,
    input: String,
) -> Result<String, crate::ComponentError> {
    fetch_text(
        execution_context,
        &format!("component://{}", handle),
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

pub fn run_json(
    execution_context: &ComponentExecutionContext,
    handle: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, crate::ComponentError> {
    let handle = ComponentHandle::from_str(&handle).map_err(|e| {
        crate::ComponentError::for_error(
            format!(
                "Failed to parse component handle \"{}\" from \"{}\"",
                handle,
                execution_context.call_chain.component_handle_trail(),
            ),
            Some(format!("{e}")),
        )
    })?;

    run_component_callout(execution_context, &handle, input).map_err(Into::into)
}

pub fn load_bin(
    execution_context: &ComponentExecutionContext,
    handle: String,
    path: String,
) -> Result<Vec<u8>, crate::ComponentError> {
    fetch_bin(
        execution_context,
        &format!("component://{}/{}", handle, path),
        None,
    )
    .map(|v| v.body)
    .map_err(Into::into)
}

pub fn load_text(
    execution_context: &ComponentExecutionContext,
    handle: String,
    path: String,
) -> Result<String, crate::ComponentError> {
    fetch_text(
        execution_context,
        &format!("component://{}/{}", handle, path),
        None,
    )
    .map(|v| v.body)
    .map_err(Into::into)
}

pub fn env(execution_context: &ComponentExecutionContext, key: &str) -> Option<String> {
    match fetch_bin(execution_context, &format!("env://{}", key), None) {
        Ok(v) => Some(String::from_utf8_lossy(&v.body).into_owned()),
        Err(e) => {
            if let Some(response) = e.response {
                if response.status_code == 404 {
                    return None;
                }
            }

            warn!(
                "Failed to fetch environment variable \"{}\" for component \"{}\":",
                key,
                execution_context.call_chain.component_handle_trail(),
            );

            warn!("{}", e.message);

            for i in e.inner {
                warn!("{i}");
            }

            None
        }
    }
}
