use slipway_engine::{ComponentExecutionContext, ComponentHandle};
use url::Url;

use crate::run::run_component_callout;

use super::{apply_json_change, RequestError, RequestOptions, Response};

pub(super) fn run_component_from_url(
    execution_context: &ComponentExecutionContext,
    handle: ComponentHandle,
    url: &Url,
    options: Option<RequestOptions>,
) -> Result<Response, RequestError> {
    let mut input = options
        .unwrap_or_default()
        .body
        .map_or_else(
            || Ok(serde_json::Value::Object(Default::default())),
            |v| serde_json::from_slice(&v),
        )
        .map_err(|e| {
            RequestError::for_message(format!(
                "Failed to parse body as JSON for component {}, url: {}\n{:#?}",
                execution_context.call_chain.component_handle_trail(),
                url,
                e
            ))
        })?;

    for (path, value) in url.query_pairs() {
        let value = if let Ok(float_value) = value.parse::<f64>() {
            serde_json::Value::Number(serde_json::Number::from_f64(float_value).unwrap())
        } else if let Ok(bool_value) = value.parse::<bool>() {
            serde_json::Value::Bool(bool_value)
        } else {
            serde_json::Value::String(value.into_owned())
        };

        apply_json_change::apply(&mut input, path.as_ref(), value);
    }

    let result_str = run_component_callout(execution_context, handle, input)?;

    Ok(Response {
        status: 200,
        headers: vec![],
        body: result_str.into_bytes(),
    })
}
