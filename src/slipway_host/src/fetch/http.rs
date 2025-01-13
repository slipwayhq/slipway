use std::io::Read;

use url::Url;

use crate::fetch::{RequestError, RequestOptions, Response};

pub(super) fn fetch_http(
    url: Url,
    options: Option<RequestOptions>,
) -> Result<Response, RequestError> {
    let opts = options.unwrap_or_default();

    let mut agent_builder = ureq::AgentBuilder::new();
    if let Some(ms) = opts.timeout_ms {
        let timeout = std::time::Duration::from_millis(ms as u64);
        agent_builder = agent_builder
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .timeout_write(timeout);
    }
    let agent = agent_builder.build();

    let mut request = agent.request_url(opts.method.as_deref().unwrap_or("GET"), &url);
    if let Some(headers) = &opts.headers {
        for (name, value) in headers {
            request = request.set(name, value);
        }
    }

    let response = match &opts.body {
        Some(body) => request.send_bytes(body),
        None => request.call(),
    };

    match response {
        Ok(response) => {
            let status = response.status();
            let mut headers = vec![];
            for name in response.headers_names() {
                if let Some(value) = response.header(&name) {
                    headers.push((name.to_string(), value.to_string()));
                }
            }
            let mut body = vec![];
            response
                .into_reader()
                .read_to_end(&mut body)
                .map_err(|e| RequestError {
                    message: format!("Reading response failed: {}", e),
                    response: None,
                })?;

            Ok(Response {
                status,
                headers,
                body,
            })
        }
        Err(err) => {
            let message = err.to_string();
            if let ureq::Error::Status(code, resp) = err {
                let mut headers = vec![];
                for name in resp.headers_names() {
                    if let Some(value) = resp.header(&name) {
                        headers.push((name.to_string(), value.to_string()));
                    }
                }
                let mut body = vec![];
                if let Err(e) = resp.into_reader().read_to_end(&mut body) {
                    return Err(RequestError {
                        message: format!("Reading error response failed: {}", e),
                        response: None,
                    });
                }
                return Err(RequestError {
                    message,
                    response: Some(Response {
                        status: code,
                        headers,
                        body,
                    }),
                });
            }
            Err(RequestError {
                message,
                response: None,
            })
        }
    }
}
