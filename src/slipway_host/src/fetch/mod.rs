mod component;
mod http;

use slipway_engine::ComponentExecutionContext;
use url::Url;

#[derive(Debug, Default)]
pub struct RequestOptions {
    pub method: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct BinResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TextResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug)]
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
            status: r.status,
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
        "https" | "http" => {
            crate::permissions::ensure_can_fetch_url(url_str, &url, execution_context)?;
            http::fetch_http(url, options)
        }
        "component" => {
            // TODO: crate::permissions::ensure_can_use_component(handle, execution_context);
            component::fetch_component_data(execution_context, &url, options)
        }
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

pub fn run(
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
