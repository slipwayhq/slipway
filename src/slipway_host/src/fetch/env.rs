use std::env::VarError;

use slipway_engine::ComponentExecutionContext;
use url::Url;

use super::{BinResponse, RequestError};

pub(super) fn fetch_env_url(
    execution_context: &ComponentExecutionContext,
    url: &Url,
) -> Result<BinResponse, RequestError> {
    let key = url.domain().ok_or_else(|| {
        RequestError::message(format!(
            "No domain (env key) found in url from component \"{}\": {}",
            execution_context.call_chain.component_handle_trail(),
            url
        ))
    })?;

    fetch_env(execution_context, key)
}

pub(super) fn fetch_env(
    execution_context: &ComponentExecutionContext,
    key: &str,
) -> Result<BinResponse, RequestError> {
    crate::permissions::ensure_can_fetch_env(key, execution_context)?;

    match std::env::var(key) {
        Ok(value) => Ok(BinResponse {
            status_code: 200,
            headers: vec![],
            body: value.into_bytes(),
        }),
        Err(VarError::NotPresent) => Ok(BinResponse {
            status_code: 404,
            headers: vec![],
            body: vec![],
        }),
        Err(e) => Err(RequestError::for_error(
            format!(
                "Failed to fetch environment variable for key \"{}\" and component \"{}\".",
                key,
                execution_context.call_chain.component_handle_trail()
            ),
            e,
        )),
    }
}
