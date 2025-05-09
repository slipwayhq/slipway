use serde::Serialize;

wit_bindgen::generate!({
    world: "slipway",
});

struct Component;

impl Guest for Component {
    fn run(input: String) -> Result<String, ComponentError> {
        let input: serde_json::Value =
            serde_json::from_str(&input).map_err(|e| ComponentError {
                message: format!("{e:#?}"),
                inner: vec![],
            })?;

        let output = Output {
            tz: std::env::var("TZ").ok(),
            lc: std::env::var("LC").ok(),
            input,
        };

        Ok(serde_json::to_string(&output).expect("Result should be serializable"))
    }
}

export!(Component);

#[derive(Serialize)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    tz: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    lc: Option<String>,

    input: serde_json::Value,
}
