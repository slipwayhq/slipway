use serde::{Deserialize, Serialize};

wit_bindgen::generate!({
    // the name of the world in the `*.wit` input file
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
    match input {
        Input::Increment { value } => {
            slipway_host::log_error("This is an error.");
            slipway_host::log_warn("This is a warning.");
            slipway_host::log_info("This is information.");
            slipway_host::log_debug("This is debug information.");
            slipway_host::log_trace("This is trace information.");
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
                    LeafCalloutResultType::Increment => Input::Increment { value },
                    LeafCalloutResultType::Panic => Input::Panic,
                    LeafCalloutResultType::Error => Input::Error,
                })
            } else {
                let callout_input = Input::CalloutIncrement {
                    value: perform_action(value),
                    ttl: ttl - 1,
                    result_type,
                };
                slipway_host::run(
                    "increment",
                    &serde_json::to_string(&callout_input).expect("should serialize output"),
                )
            }
        }
        Input::InvalidCalloutInput => slipway_host::run("increment", r#"{ "type": "foo" }"#),
        Input::InvalidCalloutOutput => {
            slipway_host::run("increment", r#"{ "type": "invalid_output" }"#)
        }
        Input::InvalidOutput => Ok(r#"{ "value": "foo" }"#.to_string()),
        Input::Panic => panic!("slipway-increment-component-panic"),
        Input::Error => Err(ComponentError {
            message: "slipway-increment-component-error".to_string(),
            inner: vec![],
        }),
    }
}

#[cfg(feature = "increment-ten")]
fn perform_action(value: i32) -> i32 {
    value + 10
}

#[cfg(not(feature = "increment-ten"))]
fn perform_action(value: i32) -> i32 {
    value + 1
}

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

    #[allow(clippy::enum_variant_names)]
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

#[derive(Serialize)]
struct Output {
    value: i32,
}
