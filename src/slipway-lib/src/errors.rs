use std::sync::Arc;

use thiserror::Error;

use crate::{execute::load_components::LoaderId, ComponentHandle, SlipwayReference};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("app parse failed")]
    ParseFailed(#[from] serde_json::Error),

    #[error("invalid json path ({0}): {1}")]
    InvalidJsonPathExpression(String, String),

    #[error("validation failed: {0}")]
    AppValidationFailed(String),

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

    #[error("component load failed for \"{0}\": {1:?}")]
    ComponentLoadFailed(ComponentHandle, Vec<ComponentLoaderFailure>),

    #[error("component input validation failed for \"{0}\": {1:?}")]
    ComponentInputValidationFailed(ComponentHandle, jtd::ValidateError),

    #[error("component output validation failed for \"{0}\": {1:?}")]
    ComponentOutputValidationFailed(ComponentHandle, jtd::ValidateError),
}

#[derive(Error, Debug, Clone)]
#[error("component load failed for {reference}: {error}")]
pub enum ComponentError {
    // We're using Arc so that ComponentError can be cloned.
    #[error("component parse failed")]
    ParseFailed(#[from] Arc<serde_json::Error>),

    // We're using Arc so that ComponentError can be cloned.
    #[error("component schema parse failed")]
    SchemaParseFailed(#[from] jtd::FromSerdeSchemaError),

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
