use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlipwayError {
    #[error("Rigging parse failed")]
    RiggingParseFailed(#[from] serde_json::Error),

    #[error("Rigging validation failed: {0}")]
    RiggingValidationFailed(String),

    #[error("Invalid component reference: {0}")]
    InvalidComponentReference(String),
}
