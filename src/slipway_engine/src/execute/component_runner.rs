use crate::{errors::ComponentLoadError, ComponentHandle, RunMetadata};
use thiserror::Error;

use super::component_execution_data::ComponentExecutionData;

pub enum TryRunComponentResult {
    CannotRun,
    Ran { result: RunComponentResult },
}

pub struct RunComponentResult {
    pub output: serde_json::Value,
    pub metadata: RunMetadata,
}

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

pub trait ComponentRunner<'rig>: Send + Sync {
    fn identifier(&self) -> String;

    fn run(
        &self,
        handle: &ComponentHandle,
        execution_data: &ComponentExecutionData<'rig>,
    ) -> Result<TryRunComponentResult, RunComponentError>;
}
