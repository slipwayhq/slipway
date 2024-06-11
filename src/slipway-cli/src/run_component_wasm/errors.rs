use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WasmExecutionError {
    #[error("unexpected failure: {0}")]
    GenericError(#[from] anyhow::Error),

    #[error("wasm step call not found")]
    StepCallNotFound(),

    #[error("wasm step call had an unexpected signature: {0}")]
    StepCallUnexpectedSignature(anyhow::Error),

    #[error("wasm step call failed: {0}\n{1:?}")]
    StepCallFailed(String, Option<anyhow::Error>),

    #[error("serializing input JSON failed: {0}")]
    SerializeInputFailed(serde_json::Error),

    #[error("deserializing output JSON failed: {0}")]
    DeserializeOutputFailed(serde_json::Error),
}
