#[allow(warnings)]
mod bindings;

use bindings::Guest;

use serde::{Deserialize, Serialize};

struct Component;

impl Guest for Component {
    /// Say hello!
    fn step(input: String) -> Result<String, String> {
        let input: Input = serde_json::from_str(&input).expect("should parse JSON from stdin");

        match input {
            Input::Increment { value } => {
                println!("error: This is an error.");
                bindings::log::error("This is an error.");
                println!("warn: This is a warning.");
                bindings::log::warn("This is a warning.");
                println!("info: This is information.");
                bindings::log::info("This is information.");
                println!("debug: This is debug information.");
                bindings::log::debug("This is debug information.");
                println!("trace: This is trace information.");
                bindings::log::trace("This is trace information.");
                println!("This is more information.");
                let output = Output { value: value + 1 };
                Ok(serde_json::to_string(&output).expect("Result should be serializable"))
            }
            Input::Panic => panic!("slipway-test-component-panic"),
            Input::Error => Err("slipway-test-component-error".to_string()),
        }
    }
}

bindings::export!(Component with_types_in bindings);

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Input {
    #[serde(rename = "increment")]
    Increment { value: i32 },

    #[serde(rename = "panic")]
    Panic,

    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize)]
struct Output {
    value: i32,
}
