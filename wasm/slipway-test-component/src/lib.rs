use std::io::Read;

use serde::{Deserialize, Serialize};

#[no_mangle]
pub fn step() {
    let mut input_string = String::new();

    std::io::stdin()
        .read_to_string(&mut input_string)
        .expect("should read from stdin");

    let input: Input = serde_json::from_str(&input_string).expect("should parse JSON from stdin");

    match input {
        Input::Increment { value } => {
            let output = Output { value: value + 1 };
            println!(
                "{}",
                serde_json::to_string(&output).expect("Result should be serializable")
            );
        }
        Input::Panic => panic!("slipway-test-component-panic"),
        Input::Stderr => {
            eprintln!("slipway-test-component-stderr");
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Input {
    #[serde(rename = "increment")]
    Increment { value: i32 },

    #[serde(rename = "panic")]
    Panic,

    #[serde(rename = "stderr")]
    Stderr,
}

#[derive(Serialize)]
struct Output {
    value: i32,
}
