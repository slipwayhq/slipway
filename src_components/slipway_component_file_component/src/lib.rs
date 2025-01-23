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
    let output = match input.file_type {
        DataResultType::Text => {
            let text = slipway_host::load_text(&input.handle, &input.path)?;
            Output {
                text: Some(text),
                bin: None,
            }
        }
        DataResultType::Binary => {
            let bin = slipway_host::load_bin(&input.handle, &input.path)?;
            Output {
                text: None,
                bin: Some(bin),
            }
        }
    };
    Ok(serde_json::to_string(&output).expect("Result should be serializable"))
}

#[derive(Serialize, Deserialize)]
struct Input {
    handle: String,
    path: String,
    file_type: DataResultType,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DataResultType {
    Text,
    Binary,
}

#[derive(Serialize)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    bin: Option<Vec<u8>>,
}
