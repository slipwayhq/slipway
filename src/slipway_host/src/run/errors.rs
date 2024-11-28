use slipway_engine::errors::ComponentLoadError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunComponentError {
    #[error("Execution error.\n{0}")]
    GenericError(#[from] anyhow::Error),

    #[error("Execution error.\n{0}")]
    Other(String),

    #[error("Component run call failed.\nAdditional details: {source:?}")]
    RunCallFailed { source: Option<anyhow::Error> },

    #[error("Component returned an error: {error}")]
    RunCallReturnedError { error: String },

    #[error("Serializing input JSON failed.\n{source}")]
    SerializeInputFailed { source: serde_json::Error },

    #[error("Deserializing output JSON failed.\n{source}")]
    DeserializeOutputFailed { source: serde_json::Error },

    #[error("Component load failed.\n{0}")]
    ComponentLoadFailed(#[from] ComponentLoadError),
}
