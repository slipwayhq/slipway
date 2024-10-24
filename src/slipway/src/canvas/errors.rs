use std::path::PathBuf;

use image::ImageError;
use slipway_lib::ComponentHandle;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CanvasError {
    #[error("Component {handle} canvas invalid: {message}")]
    InvalidData {
        handle: ComponentHandle,
        message: String,
    },

    #[error("Component {handle} canvas output image could not be saved to {path}\n{error}")]
    SaveFailed {
        handle: ComponentHandle,
        path: PathBuf,
        error: ImageError,
    },

    #[error("Component {handle} canvas output image could not be printed\n{error}")]
    PrintFailed {
        handle: ComponentHandle,
        error: String, // Using String so we don't expose the ViuError
    },
}
