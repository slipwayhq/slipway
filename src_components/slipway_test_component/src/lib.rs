#[allow(warnings)]
mod bindings;

use bindings::Guest;

use serde::{Deserialize, Serialize};

struct Component;

impl Guest for Component {
    fn run(input: String) -> Result<String, String> {
        let input: Input = serde_json::from_str(&input).expect("should parse JSON from stdin");

        match input {
            Input::Increment { value } => {
                bindings::log::error("This is an error.");
                bindings::log::warn("This is a warning.");
                bindings::log::info("This is information.");
                bindings::log::debug("This is debug information.");
                bindings::log::trace("This is trace information.");
                println!("This is more information.");
                let output = Output { value: value + 1 };
                Ok(serde_json::to_string(&output).expect("Result should be serializable"))
            }
            Input::CalloutIncrement {
                handle,
                value,
                ttl,
                result_type,
            } => {
                let callout_input = if ttl == 0 {
                    match result_type {
                        ResultType::Increment => Input::Increment { value: value },
                        ResultType::Panic => Input::Panic,
                        ResultType::Error => Input::Error,
                    }
                } else {
                    Input::CalloutIncrement {
                        handle: handle.clone(),
                        value: value + 1,
                        ttl: ttl - 1,
                        result_type,
                    }
                };
                let callout_handle = handle.unwrap_or("test".to_string());
                let output = bindings::callout::run(
                    &callout_handle,
                    &serde_json::to_string(&callout_input).expect("should serialize output"),
                );
                Ok(output)
            }
            Input::Panic => panic!("slipway-test-component-panic"),
            Input::Error => Err("slipway-test-component-error".to_string()),
        }
    }
}

bindings::export!(Component with_types_in bindings);

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Input {
    #[serde(rename = "increment")]
    Increment { value: i32 },

    #[serde(rename = "callout_increment")]
    CalloutIncrement {
        handle: Option<String>,
        value: i32,
        ttl: u32,
        result_type: ResultType,
    },

    #[serde(rename = "panic")]
    Panic,

    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResultType {
    #[serde(rename = "increment")]
    Increment,

    #[serde(rename = "panic")]
    Panic,

    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct Output {
    value: i32,
}
