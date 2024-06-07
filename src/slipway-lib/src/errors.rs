use std::{fmt, sync::Arc};

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

    #[error("component definition load failed for \"{0}\": {1:?}")]
    ComponentDefinitionLoadFailed(ComponentHandle, Vec<ComponentLoaderFailure>),

    #[error("component wasm load failed for \"{0}\": {1:?}")]
    ComponentWasmLoadFailed(ComponentHandle, Vec<ComponentLoaderFailure>),

    #[error("component {1} validation failed for \"{0}\": {2:?}")]
    ComponentValidationAborted(ComponentHandle, ValidationType, jtd::ValidateError),

    #[error("component {1} validation failed for \"{0}\": {2:?}")]
    ComponentValidationFailed(ComponentHandle, ValidationType, Vec<ValidationFailure>),
}

#[derive(Error, Debug, Clone)]
#[error("component load failed for {reference}: {error}")]
pub enum ComponentLoadError {
    // We're using Arc here so that ComponentError can be cloned.
    #[error("component definition parse failed")]
    DefinitionParseFailed(#[from] Arc<serde_json::Error>),

    #[error("component schema parse failed")]
    SchemaParseFailed(#[from] jtd::FromSerdeSchemaError),

    #[error("component definition load failed for \"{reference}\": {error}")]
    DefinitionLoadFailed {
        reference: SlipwayReference,
        error: String,
    },

    #[error("component wasm load failed for \"{reference}\": {error}")]
    WasmLoadFailed {
        reference: SlipwayReference,
        error: String,
    },

    #[error("component \"{reference}\" was not found by any loader: {loader_ids:?}")]
    NotFound {
        reference: SlipwayReference,
        loader_ids: Vec<LoaderId>,
    },
}

#[derive(Clone, Debug)]
pub struct ComponentLoaderFailure {
    pub loader_id: Option<LoaderId>,
    pub error: ComponentLoadError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationType {
    Input,
    Output,
}

impl fmt::Display for ValidationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationType::Input => write!(f, "input"),
            ValidationType::Output => write!(f, "output"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationFailure {
    /// A path to the part of the instance that was rejected.
    pub instance_path: Vec<String>,

    /// A path to the part of the schema that rejected the instance.
    pub schema_path: Vec<String>,
}

impl ValidationFailure {
    pub fn instance_path_str(&self) -> String {
        self.instance_path.join(".")
    }

    pub fn schema_path_str(&self) -> String {
        self.schema_path.join(".")
    }
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "instance path: {:?}, schema path: {:?}",
            self.instance_path_str(),
            self.schema_path_str()
        )
    }
}

impl<'a> From<jtd::ValidationErrorIndicator<'a>> for ValidationFailure {
    fn from(error: jtd::ValidationErrorIndicator) -> Self {
        let (instance_path, schema_path) = error.into_owned_paths();
        ValidationFailure {
            instance_path,
            schema_path,
        }
    }
}
