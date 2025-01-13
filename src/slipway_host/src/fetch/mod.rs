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
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub struct RequestError {
    pub message: String,
    pub response: Option<Response>,
}

impl RequestError {
    pub fn for_message(message: String) -> RequestError {
        RequestError {
            message,
            response: None,
        }
    }
}

impl From<crate::ComponentError> for RequestError {
    fn from(value: crate::ComponentError) -> Self {
        RequestError::for_message(value.message)
    }
}

pub fn fetch(
    execution_context: &ComponentExecutionContext,
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<Response, RequestError> {
    let url = Url::parse(url_str).map_err(|e| {
        RequestError::for_message(format!(
            "Failed to parse URL from component {}: {}\n{:#?}",
            execution_context.call_chain.component_handle_trail(),
            url_str,
            e
        ))
    })?;
    let scheme = url.scheme();

    match scheme {
        "https" | "http" => http::fetch_http(url, options),
        "component" => component::fetch_component_data(execution_context, &url, options),
        _ => Err(RequestError::for_message(format!(
            "Unsupported URL scheme for URL from component {}: {}",
            execution_context.call_chain.component_handle_trail(),
            url_str
        ))),
    }
}
