use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WasmExecutionError {
    #[error("WASM execution error.\n{0}")]
    GenericError(#[from] anyhow::Error),

    #[error("WASM execution error.\n{0}")]
    Other(String),

    #[error("WASM run call failed.\nAdditional details: {source:?}")]
    RunCallFailed { source: Option<anyhow::Error> },

    #[error("Component returned an error: {error}")]
    RunCallReturnedError { error: String },

    #[error("Serializing input JSON failed.\n{source}")]
    SerializeInputFailed { source: serde_json::Error },

    #[error("Deserializing output JSON failed.\n{source}")]
    DeserializeOutputFailed { source: serde_json::Error },
}
