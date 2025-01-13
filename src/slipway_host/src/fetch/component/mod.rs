mod apply_json_change;
mod component_file;
mod run;

use std::str::FromStr;

use slipway_engine::{ComponentExecutionContext, ComponentHandle};
use url::Url;

use super::{RequestError, RequestOptions, Response};

pub(super) fn fetch_component_data(
    execution_context: &ComponentExecutionContext,
    url: &Url,
    options: Option<RequestOptions>,
) -> Result<Response, RequestError> {
    let handle_str = url.domain().ok_or(RequestError::for_message(format!(
        "No domain (component handle) found in url from component {}: {}",
        execution_context.call_chain.component_handle_trail(),
        url
    )))?;

    let handle = ComponentHandle::from_str(handle_str).map_err(|e| {
        RequestError::for_message(format!(
            "Failed to parse component handle \"{}\" from \"{}\":\n{}",
            handle_str,
            execution_context.call_chain.component_handle_trail(),
            e
        ))
    })?;

    let path = url.path();

    match path {
        "" | "/" => run::run_component_from_url(execution_context, handle, url, options),
        _ => component_file::get_component_file_bin(execution_context, handle, path),
    }
}
