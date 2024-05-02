use tracing::warn;

use crate::{errors::AppError, AppSession, ComponentLoaderErrorBehavior, ComponentState};

pub(super) fn validate_component_io<'app>(
    session: &'app AppSession,
    component_state: &ComponentState<'app>,
    validation_data: ValidationData,
) -> Result<(), AppError> {
    let mut component_cache = session.component_cache.borrow_mut();
    let component_reference = &component_state.rigging.component;
    let maybe_component_definition = component_cache.get_definition(component_reference);
    match &maybe_component_definition.value {
        Some(component_definition) => {
            // If the component was loaded but there were loader failures, either error or warn
            // depending on the session options.
            if !maybe_component_definition.loader_failures.is_empty() {
                match session.component_load_error_behavior {
                    ComponentLoaderErrorBehavior::ErrorAlways => {
                        return Err(AppError::ComponentLoadFailed(
                            component_state.handle.clone(),
                            maybe_component_definition.loader_failures.clone(),
                        ));
                    }
                    ComponentLoaderErrorBehavior::ErrorIfComponentNotLoaded => {
                        for loader_failure in &maybe_component_definition.loader_failures {
                            warn!(
                                "component {} was loaded but an earlier loader {} reported an error: {}",
                                component_reference,
                                loader_failure
                                    .loader_id
                                    .as_ref()
                                    .expect("loader_id should exist on all errors if component was loaded"),
                                loader_failure.error
                            );
                        }
                    }
                }
            }

            // Validate the data against either the component input or output schema.
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
