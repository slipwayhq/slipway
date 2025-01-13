#[allow(warnings)]
mod bindings;

use std::collections::HashMap;

use bindings::{ComponentError, Guest};

use serde::{Deserialize, Serialize};

struct Component;

impl Guest for Component {
    fn run(input: String) -> Result<String, ComponentError> {
        let input: Input = serde_json::from_str(&input).expect("should parse JSON from stdin");

        run_inner(input)
    }
}

fn run_inner(input: Input) -> Result<String, bindings::slipway::component::types::ComponentError> {
    match input {
        Input::Increment { value } => {
            bindings::slipway_host::log_error("This is an error.");
            bindings::slipway_host::log_warn("This is a warning.");
            bindings::slipway_host::log_info("This is information.");
            bindings::slipway_host::log_debug("This is debug information.");
            bindings::slipway_host::log_trace("This is trace information.");
            println!("This is more information.");
            let output = Output {
                value: perform_action(value),
            };
            Ok(serde_json::to_string(&output).expect("Result should be serializable"))
        }
        Input::CalloutIncrement {
            value,
            ttl,
            result_type,
        } => {
            if ttl == 0 {
                run_inner(match result_type {
                    LeafCalloutResultType::Increment => Input::Increment { value: value },
                    LeafCalloutResultType::Panic => Input::Panic,
                    LeafCalloutResultType::Error => Input::Error,
                })
            } else {
                let callout_input = Input::CalloutIncrement {
                    value: perform_action(value),
                    ttl: ttl - 1,
                    result_type,
                };
                bindings::slipway_host::run(
                    "test",
                    &serde_json::to_string(&callout_input).expect("should serialize output"),
                )
            }
        }
        Input::ComponentFile {
            handle,
            path,
            file_type,
        } => {
            let output = match file_type {
                // Check that we successfully get the file contents and that the result contains data.
                DataResultType::Text => {
                    let text = bindings::slipway_host::load_text(&handle, &path)
                        .map_err(|v| ComponentError { message: v.message })?;
                    assert!(text.len() > 0);
                    Output {
                        value: text.len() as i32,
                    }
                }
                DataResultType::Binary => {
                    let bin = bindings::slipway_host::load_bin(&handle, &path)
                        .map_err(|v| ComponentError { message: v.message })?;
                    assert!(bin.len() > 0);
                    Output {
                        value: bin.len() as i32,
                    }
                }
            };
            Ok(serde_json::to_string(&output).expect("Result should be serializable"))
        }
        Input::Http {
            url,
            method,
            headers,
            body,
            expected_status_code,
            response_type,
        } => {
            // Make an HTTP request and check the status code is the expected value.
            // Return the result body content length.
            let request_options = bindings::slipway_host::RequestOptions {
                headers: Some(headers.into_iter().collect()),
                method: Some(method),
                body: Some(body.into_bytes()),
                timeout_ms: Some(1000),
            };
            let response = match response_type {
                DataResultType::Text => {
                    bindings::slipway_host::fetch_text(&url, Some(&request_options))
                        .map(|r| (r.status, r.body.len()))
                }
                DataResultType::Binary => {
                    bindings::slipway_host::fetch_bin(&url, Some(&request_options))
                        .map(|r| (r.status, r.body.len()))
                }
            };

            let output = if expected_status_code >= 400 {
                match response {
                    Ok(_) => Err(ComponentError {
                        message: "Expected error response, got success".to_string(),
                    }),
                    Err(e) => {
                        let response_len = e.response.as_ref().unwrap().body.len();
                        assert_eq!(e.response.unwrap().status as u32, expected_status_code);
                        Ok(Output {
                            value: response_len as i32,
                        })
                    }
                }
            } else {
                match response {
                    Ok((status_code, response_len)) => {
                        assert_eq!(status_code as u32, expected_status_code);
                        Ok(Output {
                            value: response_len as i32,
                        })
                    }
                    Err(e) => Err(ComponentError {
                        message: format!("Expected successful response, got error: {:?}", e),
                    }),
                }
            }?;

            Ok(serde_json::to_string(&output).expect("Result should be serializable"))
        }
        Input::InvalidCalloutInput => bindings::slipway_host::run("test", r#"{ "type": "foo" }"#),
        Input::InvalidCalloutOutput => {
            bindings::slipway_host::run("test", r#"{ "type": "invalid_output" }"#)
        }
        Input::InvalidOutput => Ok(r#"{ "value": "foo" }"#.to_string()),
        Input::Panic => panic!("slipway-test-component-panic"),
        Input::Error => Err(ComponentError {
            message: "slipway-test-component-error".to_string(),
        }),
    }
}

#[cfg(feature = "add-ten")]
fn perform_action(value: i32) -> i32 {
    value + 10
}

#[cfg(not(feature = "add-ten"))]
fn perform_action(value: i32) -> i32 {
    value + 1
}

bindings::export!(Component with_types_in bindings);

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Input {
    Increment {
        value: i32,
    },

    CalloutIncrement {
        value: i32,
        ttl: u32,
        result_type: LeafCalloutResultType,
    },

    ComponentFile {
        handle: String,
        path: String,
        file_type: DataResultType,
    },

    Http {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: String,
        expected_status_code: u32,
        response_type: DataResultType,
    },

    InvalidCalloutInput,
    InvalidCalloutOutput,
    InvalidOutput,
    Panic,
    Error,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LeafCalloutResultType {
    Increment,
    Panic,
    Error,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DataResultType {
    Text,
    Binary,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct Output {
    value: i32,
}
