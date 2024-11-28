use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("IO error.\n{0}")]
    Io(#[from] std::io::Error),

    #[error("Canvas error.\n{0}")]
    Canvas(#[from] crate::canvas::CanvasError),
}
