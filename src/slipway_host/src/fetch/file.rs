use slipway_engine::{ComponentExecutionContext, ProcessedUrl};

use crate::fetch::{BinResponse, RequestError, RequestOptions};

use super::TextResponse;

pub(super) async fn fetch_file(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    url: ProcessedUrl,
    options: Option<RequestOptions>,
) -> Result<BinResponse, RequestError> {
    if let Some(options) = options {
        if let Some(method) = options.method {
            if method != "GET" {
                return Err(RequestError::message(format!(
                    "Unsupported method for file fetch: {method}"
                )));
            }
        }
    }

    let file_path = match url {
        slipway_engine::ProcessedUrl::AbsolutePath(path) => {
            crate::permissions::ensure_can_fetch_file(&path, execution_context)?;
            path
        }
        slipway_engine::ProcessedUrl::RelativePath(path) => {
            crate::permissions::ensure_can_fetch_file(&path, execution_context)?;
            let base_path = &execution_context.rig_session_options.base_path;
            base_path.join(path)
        }
        _ => {
            panic!("Invalid URL scheme passed to fetch_file.");
        }
    };

    let body = match tokio::fs::read(&file_path).await {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(RequestError::response(
                format!("Failed to find file: {file_path:?}"),
                TextResponse {
                    status_code: 404,
                    headers: vec![],
                    body: Default::default(),
                },
            ));
        }
        Err(e) => {
            return Err(RequestError::for_error(
                format!("Failed to read file {file_path:?}"),
                e,
            ));
        }
    };

    let bin_response = BinResponse {
        status_code: 200,
        headers: vec![],
        body: body.to_vec(),
    };

    Ok(bin_response)
}
