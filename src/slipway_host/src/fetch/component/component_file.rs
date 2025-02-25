use std::borrow::Cow;

use slipway_engine::{ComponentExecutionContext, ComponentHandle};

use super::{BinResponse, RequestError};

pub(super) async fn get_component_file_bin(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    handle: ComponentHandle,
    path: &str,
) -> Result<BinResponse, RequestError> {
    let handle_trail = || -> String {
        execution_context
            .call_chain
            .component_handle_trail_for(&handle)
    };

    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(&handle)
        .map_err(|e| {
            RequestError::for_error(
                format!(
                    "Failed to find component to load binary file at \"{}\"",
                    handle_trail(),
                ),
                e,
            )
        })?;

    let component = execution_context.component_cache.get(component_reference);

    let path = sanitize_slashes(path);

    let bin = component.files.get_bin(path.as_ref()).await.map_err(|e| {
        RequestError::for_error(
            format!(
                "Failed to load file \"{}\" file from component \"{}\"",
                path,
                handle_trail(),
            ),
            e,
        )
    })?;

    Ok(BinResponse {
        status_code: 200,
        headers: vec![],
        body: bin.to_vec(),
    })
}

fn sanitize_slashes(path: &str) -> Cow<'_, str> {
    if !path.contains("//") && !path.starts_with('/') {
        return Cow::Borrowed(path);
    }

    let mut result = String::with_capacity(path.len());
    let mut last_was_slash = false;
    for c in path.chars() {
        if c == '/' {
            if !last_was_slash && !result.is_empty() {
                result.push(c);
            }
            last_was_slash = true;
        } else {
            result.push(c);
            last_was_slash = false;
        }
    }

    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_double_slashes() {
        assert_eq!(sanitize_slashes("a"), "a");
        assert_eq!(sanitize_slashes("//a"), "a");
        assert_eq!(sanitize_slashes("a/"), "a/");
        assert_eq!(sanitize_slashes("a//"), "a/");
        assert_eq!(sanitize_slashes("a//b"), "a/b");
        assert_eq!(sanitize_slashes("a//b/"), "a/b/");
        assert_eq!(sanitize_slashes("a//b//"), "a/b/");
        assert_eq!(sanitize_slashes("a//b//c"), "a/b/c");
        assert_eq!(sanitize_slashes("//a//b//c"), "a/b/c");
    }
}
