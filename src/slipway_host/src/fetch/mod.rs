mod component;
mod env;
mod file;
mod http;

use std::{error::Error, str::FromStr};

use env::fetch_env;
use serde::{Deserialize, Serialize};
use slipway_engine::{ComponentExecutionContext, ComponentHandle, ProcessedUrl, process_url_str};
use tracing::warn;

use crate::run::run_component_callout;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BinResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestError {
    pub message: String,
    pub inner: Vec<String>,

    // The body of an error response could technically be binary, but this is so rare that
    // I would rather make the common case of it being text easier to handle, especially
    // as the body often contains useful debugging information that is easily missed.
    pub response: Option<TextResponse>,
}

impl RequestError {
    pub fn response(message: String, response: TextResponse) -> RequestError {
        RequestError {
            message,
            inner: vec![
                format!("Response status code: {}", response.status_code),
                format!("Response body: {}", response.body),
            ],
            response: Some(response),
        }
    }

    pub fn message(message: String) -> RequestError {
        RequestError {
            message,
            inner: vec![],
            response: None,
        }
    }

    pub fn for_error(message: String, error: impl Error) -> RequestError {
        RequestError {
            message,
            inner: vec![error.to_string()],
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

pub async fn fetch_bin(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    let processed_url = process_url_str(url_str).map_err(|e| {
        RequestError::for_inner(
            format!(
                "Failed to parse URL from component {}: {url_str}",
                execution_context.call_chain.component_handle_trail()
            ),
            vec![e],
        )
    })?;

    match processed_url {
        ProcessedUrl::AbsolutePath(_) | ProcessedUrl::RelativePath(_) => {
            file::fetch_file(execution_context, processed_url, options).await
        }
        ProcessedUrl::Http(url) => http::fetch_http(execution_context, url, options).await,
        ProcessedUrl::Other(url) => match url.scheme() {
            "component" => component::fetch_component_data(execution_context, &url, options).await,
            "env" => env::fetch_env_url(execution_context, &url),
            _ => Err(RequestError::message(format!(
                "Unsupported URL scheme for URL from component {}: {}",
                execution_context.call_chain.component_handle_trail(),
                url_str
            ))),
        },
    }
}

pub async fn fetch_text(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<TextResponse, RequestError> {
    fetch_bin(execution_context, url_str, options)
        .await
        .map(Into::into)
}

pub async fn run_string(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
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
    .await
    .map(|v| v.body)
    .map_err(Into::into)
}

pub async fn run_json(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
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

    // Rather than going through fetch_bin system, which would involve serializing and deserializing
    // the input and output unnecessarily, we directly call `run_component_callout`.
    // We must therefore perform our own permissions check here.
    crate::permissions::ensure_can_use_component_handle(&handle, execution_context)?;

    run_component_callout(execution_context, &handle, input).await
}

pub async fn load_bin(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    handle: String,
    path: String,
) -> Result<Vec<u8>, crate::ComponentError> {
    fetch_bin(
        execution_context,
        &format!("component://{}/{}", handle, path),
        None,
    )
    .await
    .map(|v| v.body)
    .map_err(Into::into)
}

pub async fn load_text(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    handle: String,
    path: String,
) -> Result<String, crate::ComponentError> {
    fetch_text(
        execution_context,
        &format!("component://{}/{}", handle, path),
        None,
    )
    .await
    .map(|v| v.body)
    .map_err(Into::into)
}

pub fn env(execution_context: &ComponentExecutionContext, key: &str) -> Option<String> {
    match fetch_env(execution_context, key) {
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
