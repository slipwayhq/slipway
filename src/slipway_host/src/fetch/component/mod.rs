mod apply_json_change;
mod component_file;
mod run;

use std::str::FromStr;

use slipway_engine::{ComponentExecutionContext, ComponentHandle};
use url::Url;

use super::{BinResponse, RequestError, RequestOptions};

pub(super) async fn fetch_component_data(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    url: &Url,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    let handle = match url.domain() {
        Some(handle_str) => ComponentHandle::from_str(handle_str)
            .map(Some)
            .map_err(|e| {
                RequestError::for_error(
                    format!(
                        "Failed to parse component handle \"{}\" from \"{}\"",
                        handle_str,
                        execution_context.call_chain.component_handle_trail(),
                    ),
                    e,
                )
            }),
        None => Ok(None),
    }?;

    if let Some(handle) = &handle {
        // None implies we're using the current component, and so we don't need to check permissions.
        crate::permissions::ensure_can_use_component_handle(handle, execution_context)?;
    }

    let path = url.path();

    match path {
        "" | "/" => {
            let Some(handle) = handle else {
                return Err(RequestError::for_inner(
                    "Empty component handles are not currently supported for callouts.".to_string(),
                    vec![],
                ));
            };
            run::run_component_from_url(execution_context, handle, url, options).await
        }
        _ => component_file::get_component_file_bin(execution_context, handle, path).await,
    }
}
