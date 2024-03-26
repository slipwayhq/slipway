use std::sync::Arc;

use thiserror::Error;

use crate::{execute::load_components::LoaderId, SlipwayReference};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("app parse failed")]
    ParseFailed(#[from] serde_json::Error),

    #[error("invalid json path ({0}): {1}")]
    InvalidJsonPathExpression(String, String),

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    #[error("step failed: {0}")]
    StepFailed(String),

    #[error("resolve json path failed: {message}, state: {state:#}")]
    ResolveJsonPathFailed {
        message: String,
        state: serde_json::Value,
    },

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a ParseFailed error.
    #[error("invalid type \"{0}\": {1}")]
    InvalidSlipwayPrimitive(String, String),

    #[error("component load failed: {0:?}")]
    ComponentLoadFailed(Vec<ComponentLoaderFailure>),
}

#[derive(Error, Debug, Clone)]
#[error("component load failed for {reference}: {error}")]
pub enum ComponentError {
    #[error("component parse failed")]
    ParseFailed(#[from] Arc<serde_json::Error>),

    #[error("load failed for \"{reference}\": {error}")]
    LoadFailed {
        reference: SlipwayReference,
        error: String,
    },
}

#[derive(Clone, Debug)]
pub struct ComponentLoaderFailure {
    pub loader_id: LoaderId,
    pub error: ComponentError,
}
