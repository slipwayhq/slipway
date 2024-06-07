use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WasmExecutionError {
    #[error("executing wasm failed: {0}")]
    GenericError(#[from] anyhow::Error),
}
