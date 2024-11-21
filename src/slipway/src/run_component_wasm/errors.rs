use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WasmExecutionError {
    #[error("WASM execution error.\n{0}")]
    GenericError(#[from] anyhow::Error),

    #[error("WASM execution error.\n{0}")]
    Other(String),

    #[error("WASM run function not found.")]
    RunFunctionNotFound(),

    #[error("WASM run function had an unexpected signature.\n{source}")]
    RunFunctionUnexpectedSignature { source: anyhow::Error },

    #[error("WASM run call failed: {message}\nAdditional details: {source:?}")]
    RunCallFailed {
        message: String,
        source: Option<anyhow::Error>,
    },

    #[error("WASM run call returned an error: {error}")]
    RunCallReturnedError { error: String },

    #[error("Serializing input JSON failed.\n{source}")]
    SerializeInputFailed { source: serde_json::Error },

    #[error("Deserializing output JSON failed.\n{source}")]
    DeserializeOutputFailed { source: serde_json::Error },
}
