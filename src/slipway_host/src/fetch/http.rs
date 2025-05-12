use reqwest::{Client, ClientBuilder};
use slipway_engine::ComponentExecutionContext;
use std::time::Duration;
use url::Url;

use crate::fetch::{BinResponse, RequestError, RequestOptions};

pub(super) async fn fetch_http(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    url: Url,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    crate::permissions::ensure_can_fetch_url(&url, execution_context)?;

    let opts = options.unwrap_or_default();

    let mut client_builder = ClientBuilder::new();
    if let Some(ms) = opts.timeout_ms {
        client_builder = client_builder.timeout(Duration::from_millis(ms as u64));
    }
    let client: Client = client_builder
        .build()
        .map_err(|e| RequestError::for_error("Failed to build HTTP client.".to_string(), e))?;

    let mut request_builder = client.request(
        opts.method
            .as_deref()
            .unwrap_or("GET")
            .parse()
            .map_err(|e| RequestError::for_error("Invalid HTTP method.".to_string(), e))?,
        url,
    );

    request_builder = request_builder.header(
        "User-Agent",
        format!(
            "Slipway/{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY")
        ),
    );

    if let Some(headers) = &opts.headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }

    if let Some(body) = &opts.body {
        request_builder = request_builder.body(body.clone());
    }

    let response = request_builder
        .send()
        .await
        .map_err(|e| RequestError::for_error("HTTP request failed.".to_string(), e))?;

    let status = response.status();
    let mut headers = vec![];
    for (key, value) in response.headers().iter() {
        let val_str = value.to_str().map_err(|e| {
            RequestError::for_error("Failed to convert header to string.".to_string(), e)
        })?;
        headers.push((key.to_string(), val_str.to_string()));
    }

    let body = response.bytes().await.map_err(|e| {
        RequestError::for_error("Reading HTTP response body failed.".to_string(), e)
    })?;

    let bin_response = BinResponse {
        status_code: status.as_u16(),
        headers,
        body: body.to_vec(),
    };

    if status.is_success() {
        Ok(bin_response)
    } else {
        Err(RequestError::response(
            "Response status code did not indicate success.".to_string(),
            bin_response.into(),
        ))
    }
}
