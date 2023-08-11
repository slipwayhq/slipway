use thiserror::Error;

mod rigging;

#[derive(Error, Debug)]
pub enum SlipwayError {
    #[error("Invalid rigging: {0}")]
    InvalidRigging(String),
}
