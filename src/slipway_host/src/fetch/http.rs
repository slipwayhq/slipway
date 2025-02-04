use std::io::Read;

use slipway_engine::ComponentExecutionContext;
use ureq::{http::Request, Agent};
use url::Url;

use crate::fetch::{BinResponse, RequestError, RequestOptions};

pub(super) fn fetch_http(
    execution_context: &ComponentExecutionContext,
    url: Url,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    crate::permissions::ensure_can_fetch_url(&url, execution_context)?;

    let opts = options.unwrap_or_default();

    let mut agent_builder = ureq::Agent::config_builder().http_status_as_error(false);
    if let Some(ms) = opts.timeout_ms {
        let timeout = Some(std::time::Duration::from_millis(ms as u64));
        agent_builder = agent_builder.timeout_global(timeout);
    }
    let agent: Agent = agent_builder.build().into();

    let mut request_builder = Request::builder()
        .method(opts.method.as_deref().unwrap_or("GET"))
        .uri(url.as_str());

    if let Some(headers) = &opts.headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }

    let response = match &opts.body {
        Some(body) => {
            let request = request_builder.body(body).map_err(|e| RequestError {
                message: "Creating HTTP request with body failed.".to_string(),
                inner: vec![format!("{}", e)],
                response: None,
            })?;
            agent.run(request)
        }
        None => {
            let request = request_builder.body(()).map_err(|e| RequestError {
                message: "Creating HTTP request failed.".to_string(),
                inner: vec![format!("{}", e)],
                response: None,
            })?;
            agent.run(request)
        }
    };

    match response {
        Ok(response) => {
            let status = response.status();
            let mut headers = vec![];
            for (name, value) in response.headers() {
                headers.push((
                    name.to_string(),
                    value
                        .to_str()
                        .map_err(|e| RequestError {
                            message: "Failed to convert response header value to string."
                                .to_string(),
                            inner: vec![format!("{}", e)],
                            response: None,
                        })?
                        .to_owned(),
                ));
            }
            let mut body = vec![];
            response
                .into_body()
                .into_reader()
                .read_to_end(&mut body)
                .map_err(|e| RequestError {
                    message: "Reading HTTP response body failed.".to_string(),
                    inner: vec![format!("{}", e)],
                    response: None,
                })?;

            let bin_response = BinResponse {
                status_code: status.as_u16(),
                headers,
                body,
            };

            if status.is_success() {
                Ok(bin_response)
            } else {
                Err(RequestError {
                    message: "Response status code did not indicate success.".to_string(),
                    inner: vec![],
                    response: Some(bin_response),
                })
            }
        }
        Err(err) => Err(RequestError {
            message: err.to_string(),
            inner: vec![],
            response: None,
        }),
    }
}
