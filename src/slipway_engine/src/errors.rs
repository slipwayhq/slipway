use std::{fmt, sync::Arc};

use jsonpath_rust::parser::JsonPathParserError;
use jsonschema::error::ValidationErrorKind;
use thiserror::Error;

use crate::{ComponentHandle, SlipwayReference};

#[derive(Error, Debug)]
pub enum RigError {
    #[error("Rig definition parse failed.\n{error}")]
    RigParseFailed { error: serde_json::Error },

    #[error("Invalid JSONPath expression at location \"{location}\".\n{error}")]
    InvalidJsonPathExpression {
        location: String,
        error: JsonPathParserError,
    },

    #[error("Rig validation failed: {error}")]
    RigValidationFailed { error: String },

    #[error("Step failed: {error}")]
    StepFailed { error: String },

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
        "Component {validation_type} validation failed for \"{component_handle}\".\n{validation_error}"
    )]
    ComponentValidationAborted {
        component_handle: ComponentHandle,
        validation_type: ValidationType,
        #[source]
        validation_error: jtd::ValidateError,
    },

    #[error(
        "Component {validation_type} validation failed for \"{component_handle}\".\nData:\n{validated_data:#}\nFailures:\n{validation_failures:#?}"
    )]
    ComponentValidationFailed {
        component_handle: ComponentHandle,
        validation_type: ValidationType,
        validation_failures: SchemaValidationFailures,
        validated_data: Box<serde_json::Value>, // Boxed to keep the size of RigError small.
    },

    #[error("Rig component load failed.\n{0}")]
    ComponentLoadFailed(#[from] ComponentLoadError),

    #[error("Component not found for handle \"{handle}\".")]
    ComponentNotFound { handle: ComponentHandle },
}

#[derive(Debug)]
pub enum SchemaValidationFailures {
    JsonTypeDef(Vec<JsonTypeDefValidationFailure>),
    JsonSchema(Vec<JsonSchemaValidationFailure>),
}

#[derive(Error, Debug, Clone)]
#[error("component load failed for {reference}: {error}")]
pub struct ComponentLoadError {
    pub reference: Box<SlipwayReference>,
    pub error: ComponentLoadErrorInner,
}

impl ComponentLoadError {
    pub fn new(reference: &SlipwayReference, error: ComponentLoadErrorInner) -> Self {
        Self {
            reference: Box::new(reference.clone()),
            error,
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum ComponentLoadErrorInner {
    #[error("Component definition parse failed.\n{error}")]
    DefinitionParseFailed {
        error: Arc<serde_json::Error>, // We're using Arc here so that ComponentError can be cloned.
    },

    #[error("JSON TypeDef parse failed for {schema_name}.\n{error}")]
    JsonTypeDefParseFailed {
        schema_name: String,
        error: Arc<serde_json::Error>,
    },

    #[error("JSON TypeDef conversion failed for {schema_name}.\n{error}")]
    JsonTypeDefConversionFailed {
        schema_name: String,
        error: jtd::FromSerdeSchemaError,
    },

    #[error("JSON Schema parse failed for {schema_name}.\n{error:#?}")]
    JsonSchemaParseFailed {
        schema_name: String,
        error: JsonSchemaValidationFailure,
    },

    #[error("Component file load failed:\n{path}\n{error}")]
    FileLoadFailed { path: String, error: String },

    #[error("Component JSON file parse failed:\n{path}\n{error}")]
    FileJsonParseFailed {
        path: String,
        error: Arc<serde_json::Error>, // We're using Arc here so that ComponentError can be cloned.
    },

    #[error("Component was not found.")]
    NotFound,

    #[error("Rig has insufficient permissions to load component:\n{message}")]
    PermissionDenied { message: String, inner: Vec<String> },

    #[error("Failed to join threads.\n{message}")]
    ThreadJoinFailed { message: String },
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

pub trait SchemaValidationFailure {
    fn instance_path(&self) -> String;
    fn schema_path(&self) -> String;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonTypeDefValidationFailure {
    /// A path to the part of the instance that was rejected.
    pub instance_path: Vec<String>,

    /// A path to the part of the schema that rejected the instance.
    pub schema_path: Vec<String>,
}

impl SchemaValidationFailure for JsonTypeDefValidationFailure {
    fn instance_path(&self) -> String {
        format!("/{}", self.instance_path.join("/"))
    }

    fn schema_path(&self) -> String {
        format!("/{}", self.schema_path.join("/"))
    }
}

impl fmt::Display for JsonTypeDefValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Instance path: {}\nSchema path: {}",
            self.instance_path(),
            self.schema_path()
        )
    }
}

impl From<jtd::ValidationErrorIndicator<'_>> for JsonTypeDefValidationFailure {
    fn from(error: jtd::ValidationErrorIndicator) -> Self {
        let (instance_path, schema_path) = error.into_owned_paths();
        JsonTypeDefValidationFailure {
            instance_path,
            schema_path,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JsonSchemaValidationFailure {
    /// Type of validation error.
    pub kind: Arc<ValidationErrorKind>,

    /// Path to the value that failed validation.
    pub instance_path: String,

    /// Path to the JSON Schema keyword that failed validation.
    pub schema_path: String,
}

impl<'a> From<jsonschema::ValidationError<'a>> for JsonSchemaValidationFailure {
    fn from(error: jsonschema::ValidationError<'a>) -> Self {
        JsonSchemaValidationFailure {
            kind: Arc::new(error.kind),
            instance_path: error.instance_path.to_string(),
            schema_path: error.schema_path.to_string(),
        }
    }
}

impl SchemaValidationFailure for JsonSchemaValidationFailure {
    fn instance_path(&self) -> String {
        self.instance_path.clone()
    }

    fn schema_path(&self) -> String {
        self.schema_path.clone()
    }
}

impl fmt::Display for JsonSchemaValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Kind: {:#?}\nInstance path: {}\nSchema path: {}",
            self.kind,
            self.instance_path(),
            self.schema_path()
        )
    }
}
