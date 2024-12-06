#[allow(warnings)]
mod bindings;

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
                    ResultType::Increment => Input::Increment { value: value },
                    ResultType::Panic => Input::Panic,
                    ResultType::Error => Input::Error,
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
        result_type: ResultType,
    },

    InvalidCalloutInput,
    InvalidCalloutOutput,
    InvalidOutput,
    Panic,
    Error,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResultType {
    Increment,

    Panic,

    Error,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct Output {
    value: i32,
}
