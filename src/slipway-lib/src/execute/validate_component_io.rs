use crate::{
    errors::{AppError, ValidationType},
    AppSession, ComponentState,
};

pub(super) fn validate_component_io<'app>(
    session: &'app AppSession,
    component_state: &ComponentState<'app>,
    validation_data: ValidationData,
) -> Result<(), AppError> {
    let component_reference = &component_state.rigging.component;
    let component_definition = session.component_cache.get_definition(component_reference);

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
        App, ComponentCache, ComponentRigging, Instruction, Rigging,
    };

    use super::*;

    fn create_app() -> App {
        // Create a fully populated app instance.
        // Dependency graph:
        //  A
        //  |
        //  B
        App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("a", None),
                ComponentRigging::for_test("b", Some(json!({ "a_output": "$$.a" }))),
            ]
            .into_iter()
            .collect(),
        })
    }

    #[test]
    fn it_should_validate_component_input() {
        let app = create_app();

        let component_cache = ComponentCache::for_test_with_schemas(
            &app,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(json!({
                            "properties": {
                                "a_output": {
                                    "properties": {
                                        "foo": {
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

        let app_session = AppSession::new(app, component_cache);

        let mut s = app_session.initialize().unwrap();

        s = s
            .step(Instruction::SetOutput {
                handle: ch("a"),
                value: json!({ "foo": "bar" }),
            })
            .unwrap();

        let b_component_state = s.get_component_state(&ch("b")).unwrap();
        let b_execution_input = b_component_state.execution_input.as_ref().unwrap();
        assert_eq!(
            b_execution_input.value,
            json!({ "a_output": { "foo": "bar" } })
        );
    }

    #[test]
    fn it_should_fail_to_validate_invalid_component_input() {
        let app = create_app();

        let component_cache = ComponentCache::for_test_with_schemas(
            &app,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(json!({
                            "properties": {
                                "a_output": {
                                    "properties": {
                                        "foo": {
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

        let app_session = AppSession::new(app, component_cache);

        let s = app_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("a"),
            value: json!({ "foo": "bar" }),
        });

        match s_result {
            Err(AppError::ComponentValidationFailed(component_handle, validation_type, errors)) => {
                assert_eq!(component_handle, ch("b"));
                assert_eq!(validation_type, ValidationType::Input);
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].instance_path_str(), "a_output.foo");
                assert_eq!(
                    errors[0].schema_path_str(),
                    "properties.a_output.properties.foo.type"
                );
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }

    #[test]
    fn it_should_validate_component_output() {
        let app = create_app();

        let component_cache = ComponentCache::for_test_with_schemas(
            &app,
            [
                (
                    "a".to_string(),
                    (
                        schema_any(),
                        schema_valid(json!({
                            "properties": {
                                "foo": {
                                    "type": "string"
                                }
                            }
                        })),
                    ),
                ),
                ("b".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        );

        let app_session = AppSession::new(app, component_cache);

        let mut s = app_session.initialize().unwrap();
        s = s
            .step(Instruction::SetOutput {
                handle: ch("a"),
                value: json!({ "foo": "bar" }),
            })
            .unwrap();

        let b_component_state = s.get_component_state(&ch("b")).unwrap();
        let b_execution_input = b_component_state.execution_input.as_ref().unwrap();
        assert_eq!(
            b_execution_input.value,
            json!({ "a_output": { "foo": "bar" } })
        );
    }

    #[test]
    fn it_should_fail_to_validate_invalid_component_output() {
        let app = create_app();

        let component_cache = ComponentCache::for_test_with_schemas(
            &app,
            [
                (
                    "a".to_string(),
                    (
                        schema_any(),
                        schema_valid(json!({
                            "properties": {
                                "foo": {
                                    "type": "int32"
                                }
                            }
                        })),
                    ),
                ),
                ("b".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        );

        let app_session = AppSession::new(app, component_cache);

        let s = app_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("a"),
            value: json!({ "foo": "bar" }),
        });

        match s_result {
            Err(AppError::ComponentValidationFailed(component_handle, validation_type, errors)) => {
                assert_eq!(component_handle, ch("a"));
                assert_eq!(validation_type, ValidationType::Output);
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].instance_path_str(), "foo");
                assert_eq!(errors[0].schema_path_str(), "properties.foo.type");
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }
}
