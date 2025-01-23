use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use slipway_host::RequestError;

wit_bindgen::generate!({
    world: "slipway",
});

struct Component;

impl Guest for Component {
    fn run(input: String) -> Result<String, ComponentError> {
        let input: Input = serde_json::from_str(&input).map_err(|e| ComponentError {
            message: format!("{e:#?}"),
            inner: vec![],
        })?;

        run_inner(input)
    }
}

export!(Component);

fn run_inner(input: Input) -> Result<String, ComponentError> {
    let Input {
        url,
        method,
        headers,
        body,
        response_type,
    } = input;

    let request_options = slipway_host::RequestOptions {
        headers: Some(headers.into_iter().collect()),
        method: Some(method),
        body: Some(body.into_bytes()),
        timeout_ms: Some(1000),
    };

    fn map_err_to_output(e: RequestError) -> Output {
        if let Some(response) = e.response {
            Output {
                status_code: response.status_code,
                body_text: None,
                body_bin: Some(response.body),
            }
        } else {
            Output {
                status_code: 0,
                body_text: Some(e.message),
                body_bin: None,
            }
        }
    }

    let output = match response_type {
        DataResultType::Text => slipway_host::fetch_text(&url, Some(&request_options))
            .map(|r| Output {
                status_code: r.status_code,
                body_text: Some(r.body),
                body_bin: None,
            })
            .unwrap_or_else(map_err_to_output),
        DataResultType::Binary => slipway_host::fetch_bin(&url, Some(&request_options))
            .map(|r| Output {
                status_code: r.status_code,
                body_text: None,
                body_bin: Some(r.body),
            })
            .unwrap_or_else(map_err_to_output),
    };

    Ok(serde_json::to_string(&output).expect("Result should be serializable"))
}

#[derive(Serialize, Deserialize)]
struct Input {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: String,
    response_type: DataResultType,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DataResultType {
    Text,
    Binary,
}

#[derive(Serialize)]
struct Output {
    status_code: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    body_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    body_bin: Option<Vec<u8>>,
}
