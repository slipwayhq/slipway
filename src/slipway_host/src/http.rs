use std::io::Read;

#[derive(Debug)]
pub struct RequestOptions {
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug)]
pub struct TextResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct BinResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub struct RequestError {
    pub message: String,
    pub response: Option<BinResponse>,
}

pub fn request_bin(
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    let opts = options.unwrap_or_else(|| RequestOptions {
        method: "GET".to_string(),
        headers: vec![],
        body: None,
        timeout_ms: None,
    });

    let mut agent_builder = ureq::AgentBuilder::new();
    if let Some(ms) = opts.timeout_ms {
        let timeout = std::time::Duration::from_millis(ms as u64);
        agent_builder = agent_builder
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .timeout_write(timeout);
    }
    let agent = agent_builder.build();

    let mut request = agent.request(&opts.method, url_str);
    for (name, value) in &opts.headers {
        request = request.set(name, value);
    }

    let response = match &opts.body {
        Some(body) => request.send_string(body),
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

            Ok(BinResponse {
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
                    response: Some(BinResponse {
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

pub fn request_text(
    url_str: &str,
    options: Option<RequestOptions>,
) -> Result<TextResponse, RequestError> {
    let bin_result = request_bin(url_str, options)?;
    let body_string = String::from_utf8(bin_result.body.clone()).map_err(|err| RequestError {
        message: err.to_string(),
        response: Some(bin_result.clone()),
    })?;
    Ok(TextResponse {
        status: bin_result.status,
        headers: bin_result.headers,
        body: body_string,
    })
}
