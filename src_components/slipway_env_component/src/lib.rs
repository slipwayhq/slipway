use serde::{Deserialize, Serialize};

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
    let env = slipway_host::env(&input.key);
    let output = Output { value: env };
    Ok(serde_json::to_string(&output).expect("Result should be serializable"))
}

#[derive(Serialize, Deserialize)]
struct Input {
    key: String,
}

#[derive(Serialize)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}
