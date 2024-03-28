use crate::{errors::AppError, AppSession, ComponentState};

pub(super) fn validate_component_io<'app, 'data>(
    session: &'app AppSession,
    component_state: &ComponentState<'app>,
    validation_data: ValidationData<'data>,
) -> Result<(), AppError> {
    let mut component_cache = session.component_cache.borrow_mut();
    let component_definition = component_cache.get_definition(&component_state.rigging.component);
    // TODO: Validate execution input and update metadata.
    match &component_definition.value {
        Some(value) => {
            todo!("validate execution output")
        }
        None => {
            return Err(AppError::ComponentLoadFailed(
                component_state.handle.clone(),
                component_definition.loader_failures.clone(),
            ));
        }
    }
    Ok(())
}

pub(super) enum ValidationData<'data> {
    Input(&'data serde_json::Value),
    Output(&'data serde_json::Value),
}
