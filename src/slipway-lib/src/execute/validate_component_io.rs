use crate::{errors::AppError, AppSession, ComponentState};

pub(super) fn validate_component_io<'app>(
    session: &'app AppSession,
    component_state: &ComponentState<'app>,
    validation_data: ValidationData,
) -> Result<(), AppError> {
    let mut component_cache = session.component_cache.borrow_mut();
    let maybe_component_definition =
        component_cache.get_definition(&component_state.rigging.component);
    match &maybe_component_definition.value {
        Some(component_definition) => {
            if !maybe_component_definition.loader_failures.is_empty() {
                // TODO: Log warnings that errors were reported by loaders despite the component being loaded.
            }
            match validation_data {
                ValidationData::Input(input) => {
                    jtd::validate(&component_definition.input, input, Default::default()).map_err(
                        |e| {
                            AppError::ComponentInputValidationFailed(
                                component_state.handle.clone(),
                                e,
                            )
                        },
                    )?;
                }
                ValidationData::Output(output) => {
                    jtd::validate(&component_definition.output, output, Default::default())
                        .map_err(|e| {
                            AppError::ComponentOutputValidationFailed(
                                component_state.handle.clone(),
                                e,
                            )
                        })?;
                }
            }
        }
        None => {
            return Err(AppError::ComponentLoadFailed(
                component_state.handle.clone(),
                maybe_component_definition.loader_failures.clone(),
            ));
        }
    }
    Ok(())
}

pub(super) enum ValidationData<'data> {
    Input(&'data serde_json::Value),
    Output(&'data serde_json::Value),
}
