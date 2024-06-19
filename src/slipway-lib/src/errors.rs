use std::{fmt, sync::Arc};

use thiserror::Error;

use crate::{ComponentHandle, SlipwayReference};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("App definition parse failed.\n{0}")]
    ParseFailed(#[from] serde_json::Error),

    #[error("Invalid JSONPath expression at location \"{location}\".\n{message}")]
    InvalidJsonPathExpression { location: String, message: String },

    #[error("App validation failed: {0}")]
    AppValidationFailed(String),

    #[error("Step failed: {0}")]
    StepFailed(String),

    #[error("Resolve JSONPath failed: {message}\nState: {state:#}")]
    ResolveJsonPathFailed {
        message: String,
        state: serde_json::Value,
    },

    // If this error is generated during Serde deserialization it will be converted
    // into a `serde_json::Error` and wrapped in a ParseFailed error.
    #[error("Invalid {primitive_type}: {message}")]
    InvalidSlipwayPrimitive {
        primitive_type: String,
        message: String,
    },

    #[error(
        "Component {validation_type} validation failed for \"{component_handle}\".\n{validation_error:?}"
    )]
    ComponentValidationAborted {
        component_handle: ComponentHandle,
        validation_type: ValidationType,
        #[source]
        validation_error: jtd::ValidateError,
    },

    #[error(
        "Component {validation_type} validation failed for \"{component_handle}\".\nFailures:\n{validation_failures:#?}\nData:\n{validated_data:#}"
    )]
    ComponentValidationFailed {
        component_handle: ComponentHandle,
        validation_type: ValidationType,
        validation_failures: Vec<ValidationFailure>,
        validated_data: serde_json::Value,
    },
}

#[derive(Error, Debug, Clone)]
#[error("component load failed for {reference}: {error}")]
pub enum ComponentLoadError {
    // We're using Arc here so that ComponentError can be cloned.
    #[error("Component definition parse failed.\n{0}")]
    DefinitionParseFailed(#[from] Arc<serde_json::Error>),

    #[error("Component schema parse failed.\n{0}")]
    SchemaParseFailed(#[from] jtd::FromSerdeSchemaError),

    #[error("Component definition load failed for \"{reference}\"\n{error}")]
    DefinitionLoadFailed {
        reference: SlipwayReference,
        error: String,
    },

    #[error("Component WASM load failed for \"{reference}\"\n{error}")]
    WasmLoadFailed {
        reference: SlipwayReference,
        error: String,
    },

    #[error("Component \"{reference}\" was not found.")]
    NotFound { reference: SlipwayReference },
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
