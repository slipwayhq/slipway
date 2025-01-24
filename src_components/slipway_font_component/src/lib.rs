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
    let maybe_resolved_font = slipway_host::try_resolve_font(&input.font_stack);

    let output = Output {
        bin_length: maybe_resolved_font
            .map(|resolved_font| resolved_font.data.len() as u32)
            .unwrap_or(0),
    };

    Ok(serde_json::to_string(&output).expect("Result should be serializable"))
}

#[derive(Serialize, Deserialize)]
struct Input {
    font_stack: String,
}

#[derive(Serialize)]
struct Output {
    bin_length: u32,
}
