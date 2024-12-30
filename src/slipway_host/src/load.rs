use slipway_engine::ComponentExecutionContext;

use crate::{parse_handle, ComponentError};

pub fn get_component_file_text(
    execution_context: &ComponentExecutionContext,
    handle: &str,
    path: &str,
) -> Result<String, ComponentError> {
    let handle = parse_handle(execution_context, handle)?;

    let handle_trail = || -> String {
        execution_context
            .call_chain
            .component_handle_trail_for(&handle)
    };

    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(&handle)
        .map_err(|e| ComponentError {
            message: format!(
                "Failed to find component to load text file at \"{}\":\n{}",
                handle_trail(),
                e
            ),
        })?;

    let component = execution_context.component_cache.get(component_reference);

    let text = component.files.get_text(path).map_err(|e| ComponentError {
        message: format!(
            "Failed to load text file \"{}\" file from component \"{}\":\n{}",
            path,
            handle_trail(),
            e
        ),
    })?;

    Ok(text.to_string())
}

pub fn get_component_file_bin(
    execution_context: &ComponentExecutionContext,
    handle: &str,
    path: &str,
) -> Result<Vec<u8>, ComponentError> {
    let handle = parse_handle(execution_context, handle)?;

    let handle_trail = || -> String {
        execution_context
            .call_chain
            .component_handle_trail_for(&handle)
    };

    let component_reference = execution_context
        .callout_context
        .get_component_reference_for_handle(&handle)
        .map_err(|e| ComponentError {
            message: format!(
                "Failed to find component to load binary file at \"{}\":\n{}",
                handle_trail(),
                e
            ),
        })?;

    let component = execution_context.component_cache.get(component_reference);

    let bin = component.files.get_bin(path).map_err(|e| ComponentError {
        message: format!(
            "Failed to load binary file \"{}\" file from component \"{}\":\n{}",
            path,
            handle_trail(),
            e
        ),
    })?;

    Ok(bin.to_vec())
}
