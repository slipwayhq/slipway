use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WasmExecutionError {
    #[error("WASM execution error.\n{0}")]
    GenericError(#[from] anyhow::Error),

    #[error("WASM execution error.\n{0}")]
    Other(String),

    #[error("WASM step function not found.")]
    StepFunctionNotFound(),

    #[error("WASM step function had an unexpected signature.\n{source}")]
    StepFunctionUnexpectedSignature { source: anyhow::Error },

    #[error("WASM step call failed: {message}\nAdditional details: {source:?}")]
    StepCallFailed {
        message: String,
        source: Option<anyhow::Error>,
    },

    #[error("Serializing input JSON failed.\n{source}")]
    SerializeInputFailed { source: serde_json::Error },

    #[error("Deserializing output JSON failed.\n{source}")]
    DeserializeOutputFailed { source: serde_json::Error },
}
