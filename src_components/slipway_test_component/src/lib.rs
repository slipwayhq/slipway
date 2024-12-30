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
            bindings::log::error("This is an error.");
            bindings::log::warn("This is a warning.");
            bindings::log::info("This is information.");
            bindings::log::debug("This is debug information.");
            bindings::log::trace("This is trace information.");
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
                bindings::callout::run(
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
                    let text = bindings::callout::get_text(&handle, &path)?;
                    assert!(text.len() > 0);
                    Output {
                        value: text.len() as i32,
                    }
                }
                DataResultType::Binary => {
                    let bin = bindings::callout::get_bin(&handle, &path)?;
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
            let request_options = bindings::http::RequestOptions {
                headers: headers.into_iter().collect(),
                method,
                body: Some(body),
                timeout_ms: Some(1000),
            };
            let response = match response_type {
                DataResultType::Text => bindings::http::request_text(&url, Some(&request_options))
                    .map(|r| (r.status, r.body.len())),
                DataResultType::Binary => bindings::http::request_bin(&url, Some(&request_options))
                    .map(|r| (r.status, r.body.len())),
            };

            let output = if expected_status_code >= 400 {
                match response {
                    Ok(_) => {
                        panic!("Expected error response, got success");
                    }
                    Err(e) => {
                        assert_eq!(e.response.unwrap().status as u32, expected_status_code);
                        Output { value: 0 }
                    }
                }
            } else {
                match response {
                    Ok((status_code, response_len)) => {
                        assert_eq!(status_code as u32, expected_status_code);
                        Output {
                            value: response_len as i32,
                        }
                    }
                    Err(e) => {
                        panic!("Expected successful response, got error: {:?}", e);
                    }
                }
            };

            Ok(serde_json::to_string(&output).expect("Result should be serializable"))
        }
        Input::InvalidCalloutInput => bindings::callout::run("test", r#"{ "type": "foo" }"#),
        Input::InvalidCalloutOutput => {
            bindings::callout::run("test", r#"{ "type": "invalid_output" }"#)
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
