use slipway_engine::{ComponentExecutionContext, ComponentHandle};
use url::Url;

use crate::run::run_component_callout;

use super::{BinResponse, RequestError, RequestOptions, apply_json_change};

pub(super) async fn run_component_from_url(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    handle: ComponentHandle,
    url: &Url,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    let mut input = options
        .unwrap_or_default()
        .body
        .map_or_else(
            || Ok(serde_json::Value::Object(Default::default())),
            |v| serde_json::from_slice(&v),
        )
        .map_err(|e| {
            RequestError::for_error(
                format!(
                    "Failed to parse body as JSON for component {}, url: {}",
                    execution_context.call_chain.component_handle_trail(),
                    url,
                ),
                e,
            )
        })?;

    for (path, value) in url.query_pairs() {
        let value = if let Ok(int_value) = value.parse::<i64>() {
            serde_json::Value::Number(serde_json::Number::from(int_value))
        } else if let Ok(float_value) = value.parse::<f64>() {
            if float_value.is_finite() {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(float_value)
                        .expect("finite float should convert to JSON number"),
                )
            } else {
                serde_json::Value::String(value.into_owned())
            }
        } else if let Ok(bool_value) = value.parse::<bool>() {
            serde_json::Value::Bool(bool_value)
        } else {
            serde_json::Value::String(value.into_owned())
        };

        apply_json_change::apply(&mut input, path.as_ref(), value);
    }

    let result = run_component_callout(execution_context, &handle, input).await?;

    let result_bytes = serde_json::to_vec(&result).map_err(|e| {
        RequestError::for_error(
            format!(
                "Failed to serialize output JSON for callout {}, url: {}",
                execution_context
                    .call_chain
                    .component_handle_trail_for(&handle),
                url,
            ),
            e,
        )
    })?;

    Ok(BinResponse {
        status_code: 200,
        headers: vec![("content-type".to_string(), "application/json".to_string())],
        body: result_bytes,
    })
}
