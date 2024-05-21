use tracing::warn;

use crate::{
    errors::{AppError, ValidationType},
    AppSession, ComponentLoaderErrorBehavior, ComponentState,
};

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
            let (validation_type, validation_result) = match validation_data {
                ValidationData::Input(input) => (
                    ValidationType::Input,
                    jtd::validate(&component_definition.input, input, Default::default()),
                ),
                ValidationData::Output(output) => (
                    ValidationType::Output,
                    jtd::validate(&component_definition.output, output, Default::default()),
                ),
            };

            // The errors returned as part of the Result are fundamental validation errors when trying
            // to validate, rather than errors caused by the JSON not matching the schema.
            let errors = validation_result.map_err(|e| {
                AppError::ComponentValidationAborted(
                    component_state.handle.clone(),
                    validation_type.clone(),
                    e,
                )
            })?;

            // The errors returned within the result are the actual schema validation errors.
            if !errors.is_empty() {
                return Err(AppError::ComponentValidationFailed(
                    component_state.handle.clone(),
                    validation_type.clone(),
                    errors.into_iter().map(|e| e.into()).collect(),
                ));
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

#[cfg(test)]
mod tests {

    use serde_json::json;

    use crate::{
        test_utils::{schema_any, schema_valid},
        utils::ch,
        App, ComponentRigging, Instruction, Rigging,
    };

    use super::*;

    fn create_app() -> App {
        // Create a fully populated app instance.
        // Dependency graph:
        //  B
        //  |
        //  A
        App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("a", Some(json!({ "b": "$$.b" }))),
                ComponentRigging::for_test("b", None),
            ]
            .into_iter()
            .collect(),
        })
    }

    #[test]
    fn it_should_validate_component_input() {
        let app = create_app();

        let app_session = AppSession::for_test_with_schemas(
            app,
            [
                ("b".to_string(), (schema_any(), schema_any())),
                (
                    "a".to_string(),
                    (
                        schema_valid(json!({
                            "properties": {
                                "b": {
                                    "properties": {
                                        "d": {
                                            "type": "string"
                                        }
                                    }
                                },
                            }
                        })),
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let mut s = app_session.initialize().unwrap();
        s = s
            .step(Instruction::SetOutput {
                handle: ch("b"),
                value: json!({ "d": "foo" }),
            })
            .unwrap();

        let a_component_state = s.get_component_state(&ch("a")).unwrap();
        let a_execution_input = a_component_state.execution_input.as_ref().unwrap();
        assert_eq!(a_execution_input.value, json!({ "b": { "d": "foo" } }));
    }

    #[test]
    fn it_should_fail_to_validate_invalid_component_input() {
        let app = create_app();

        let app_session = AppSession::for_test_with_schemas(
            app,
            [
                ("b".to_string(), (schema_any(), schema_any())),
                (
                    "a".to_string(),
                    (
                        schema_valid(json!({
                            "properties": {
                                "b": {
                                    "properties": {
                                        "d": {
                                            "type": "int32"
                                        }
                                    }
                                },
                            }
                        })),
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let s = app_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("b"),
            value: json!({ "d": "foo" }),
        });

        match s_result {
            Err(AppError::ComponentValidationFailed(component_handle, validation_type, errors)) => {
                assert_eq!(component_handle, ch("a"));
                assert_eq!(validation_type, ValidationType::Input);
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].instance_path_str(), "b.d");
                assert_eq!(
                    errors[0].schema_path_str(),
                    "properties.b.properties.d.type"
                );
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }

    #[test]
    fn it_should_validate_component_output() {
        let app = create_app();

        let app_session = AppSession::for_test_with_schemas(
            app,
            [
                (
                    "b".to_string(),
                    (
                        schema_any(),
                        schema_valid(json!({
                            "properties": {
                                "d": {
                                    "type": "string"
                                }
                            }
                        })),
                    ),
                ),
                ("a".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        );

        let mut s = app_session.initialize().unwrap();
        s = s
            .step(Instruction::SetOutput {
                handle: ch("b"),
                value: json!({ "d": "foo" }),
            })
            .unwrap();

        let a_component_state = s.get_component_state(&ch("a")).unwrap();
        let a_execution_input = a_component_state.execution_input.as_ref().unwrap();
        assert_eq!(a_execution_input.value, json!({ "b": { "d": "foo" } }));
    }

    #[test]
    fn it_should_fail_to_validate_invalid_component_output() {
        let app = create_app();

        let app_session = AppSession::for_test_with_schemas(
            app,
            [
                (
                    "b".to_string(),
                    (
                        schema_any(),
                        schema_valid(json!({
                            "properties": {
                                "d": {
                                    "type": "int32"
                                }
                            }
                        })),
                    ),
                ),
                ("a".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        );

        let s = app_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("b"),
            value: json!({ "d": "foo" }),
        });

        match s_result {
            Err(AppError::ComponentValidationFailed(component_handle, validation_type, errors)) => {
                assert_eq!(component_handle, ch("b"));
                assert_eq!(validation_type, ValidationType::Output);
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].instance_path_str(), "d");
                assert_eq!(errors[0].schema_path_str(), "properties.d.type");
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }
}
