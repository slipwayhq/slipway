use std::sync::Arc;

use crate::{
    ComponentHandle, ComponentState, RigSession, Schema,
    errors::{RigError, SchemaValidationFailures, ValidationType},
};

pub(super) fn validate_component_io_from_session<'rig>(
    session: &'rig RigSession,
    component_state: &ComponentState<'rig>,
    validation_data: ValidationData,
) -> Result<(), RigError> {
    let component_reference = &component_state.rigging.component;
    let component_definition =
        Arc::clone(&session.component_cache.get(component_reference).definition);
    let component_handle = &component_state.handle;

    validate_component_io(validation_data, component_definition, component_handle)
}

pub fn validate_component_io(
    validation_data: ValidationData<'_>,
    component_definition: std::sync::Arc<crate::Component<Schema>>,
    component_handle: &ComponentHandle,
) -> Result<(), RigError> {
    // Validate the data against either the component input or output schema.
    let (validation_type, validation_result) = match validation_data {
        ValidationData::Input(input) => (
            ValidationType::Input,
            validate_json(&component_definition.input, input),
        ),
        ValidationData::Output(output) => (
            ValidationType::Output,
            validate_json(&component_definition.output, output),
        ),
    };

    let validation_failures = match validation_result {
        ValidationResult::JsonTypeDef(validation_result) => {
            // The errors returned as part of the Result are fundamental validation errors when trying
            // to validate, rather than errors caused by the JSON not matching the schema.
            let errors = validation_result.map_err(|e| RigError::ComponentValidationAborted {
                component_handle: component_handle.clone(),
                validation_type: validation_type.clone(),
                validation_error: e,
            })?;

            // The errors returned within the result are the actual schema validation errors.
            if !errors.is_empty() {
                Some(SchemaValidationFailures::JsonTypeDef(
                    errors.into_iter().map(|e| e.into()).collect(),
                ))
            } else {
                None
            }
        }
        ValidationResult::JsonSchema(result) => {
            if let Err(errors) = result {
                Some(SchemaValidationFailures::JsonSchema(
                    errors.into_iter().map(|e| e.into()).collect(),
                ))
            } else {
                None
            }
        }
    };

    if let Some(validation_failures) = validation_failures {
        return Err(RigError::ComponentValidationFailed {
            component_handle: component_handle.clone(),
            validation_type: validation_type.clone(),
            validation_failures,
            validated_data: match validation_data {
                ValidationData::Input(input) => Box::new(input.clone()),
                ValidationData::Output(output) => Box::new(output.clone()),
            },
        });
    }

    Ok(())
}

fn validate_json<'rig, 'data>(
    schema: &'rig Schema,
    data: &'data serde_json::Value,
) -> ValidationResult<'data>
where
    'rig: 'data,
{
    match schema {
        Schema::JsonTypeDef { schema } => {
            let validation_result = jtd::validate(schema, data, Default::default());
            ValidationResult::JsonTypeDef(validation_result)
        }
        Schema::JsonSchema {
            schema,
            original: _,
        } => {
            let validation_result = schema.iter_errors(data);
            let errors: Vec<_> = validation_result.collect();
            ValidationResult::JsonSchema(if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            })
        }
    }
}

pub enum ValidationResult<'data> {
    JsonTypeDef(Result<Vec<jtd::ValidationErrorIndicator<'data>>, jtd::ValidateError>),
    JsonSchema(Result<(), Vec<jsonschema::ValidationError<'data>>>),
}

pub(super) enum ValidationData<'data> {
    Input(&'data serde_json::Value),
    Output(&'data serde_json::Value),
}

#[cfg(test)]
mod tests {

    use common_macros::slipway_test_async;
    use serde_json::json;

    use crate::{
        BasicComponentCache, ComponentRigging, Instruction, Rig, Rigging,
        errors::SchemaValidationFailure,
        test_utils::{schema_any, schema_valid},
        utils::ch,
    };

    use super::*;

    fn create_rig() -> Rig {
        // Create a fully populated rig instance.
        // Dependency graph:
        //  A
        //  |
        //  B
        Rig::for_test(Rigging {
            components: [
                ComponentRigging::for_test("a", None),
                ComponentRigging::for_test("b", Some(json!({ "a_output": "$$.a" }))),
            ]
            .into_iter()
            .collect(),
        })
    }

    #[slipway_test_async]
    async fn it_should_validate_component_input() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(
                            "b.input",
                            json!({
                                "properties": {
                                    "a_output": {
                                        "properties": {
                                            "foo": {
                                                "type": "string"
                                            }
                                        }
                                    },
                                }
                            }),
                        )
                        .await,
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let mut s = rig_session.initialize().unwrap();

        s = s
            .step(Instruction::SetOutput {
                handle: ch("a"),
                value: json!({ "foo": "bar" }),
                metadata: Default::default(),
            })
            .unwrap();

        let b_component_state = s.get_component_state(&ch("b")).unwrap();
        let b_execution_input = b_component_state.execution_input.as_ref().unwrap();
        assert_eq!(
            b_execution_input.value,
            json!({ "a_output": { "foo": "bar" } })
        );
    }

    #[slipway_test_async]
    async fn it_should_fail_to_validate_invalid_component_input() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(
                            "b.input",
                            json!({
                                "properties": {
                                    "a_output": {
                                        "properties": {
                                            "foo": {
                                                "type": "int32"
                                            }
                                        }
                                    },
                                }
                            }),
                        )
                        .await,
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let s = rig_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("a"),
            value: json!({ "foo": "bar" }),
            metadata: Default::default(),
        });

        match s_result {
            Err(RigError::ComponentValidationFailed {
                component_handle,
                validation_type,
                validation_failures,
                validated_data,
            }) => {
                assert_eq!(component_handle, ch("b"));
                assert_eq!(validation_type, ValidationType::Input);

                match validation_failures {
                    SchemaValidationFailures::JsonTypeDef(validation_failures) => {
                        assert_eq!(validation_failures.len(), 1);
                        assert_eq!(validation_failures[0].instance_path(), "/a_output/foo");
                        assert_eq!(
                            validation_failures[0].schema_path(),
                            "/properties/a_output/properties/foo/type"
                        );
                    }
                    _ => panic!("Expected JsonTypeDef validation failures"),
                }

                assert_eq!(
                    validated_data,
                    Box::new(json!({ "a_output": { "foo": "bar" } }))
                );
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }

    #[slipway_test_async]
    async fn it_should_validate_component_output() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                (
                    "a".to_string(),
                    (
                        schema_any(),
                        schema_valid(
                            "a.output",
                            json!({
                                "properties": {
                                    "foo": {
                                        "type": "string"
                                    }
                                }
                            }),
                        )
                        .await,
                    ),
                ),
                ("b".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let mut s = rig_session.initialize().unwrap();
        s = s
            .step(Instruction::SetOutput {
                handle: ch("a"),
                value: json!({ "foo": "bar" }),
                metadata: Default::default(),
            })
            .unwrap();

        let b_component_state = s.get_component_state(&ch("b")).unwrap();
        let b_execution_input = b_component_state.execution_input.as_ref().unwrap();
        assert_eq!(
            b_execution_input.value,
            json!({ "a_output": { "foo": "bar" } })
        );
    }

    #[slipway_test_async]
    async fn it_should_fail_to_validate_invalid_component_output() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                (
                    "a".to_string(),
                    (
                        schema_any(),
                        schema_valid(
                            "a.output",
                            json!({
                                "properties": {
                                    "foo": {
                                        "type": "int32"
                                    }
                                }
                            }),
                        )
                        .await,
                    ),
                ),
                ("b".to_string(), (schema_any(), schema_any())),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let s = rig_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("a"),
            value: json!({ "foo": "bar" }),
            metadata: Default::default(),
        });

        match s_result {
            Err(RigError::ComponentValidationFailed {
                component_handle,
                validation_type,
                validation_failures,
                validated_data,
            }) => {
                assert_eq!(component_handle, ch("a"));
                assert_eq!(validation_type, ValidationType::Output);

                match validation_failures {
                    SchemaValidationFailures::JsonTypeDef(validation_failures) => {
                        assert_eq!(validation_failures.len(), 1);
                        assert_eq!(validation_failures[0].instance_path(), "/foo");
                        assert_eq!(validation_failures[0].schema_path(), "/properties/foo/type");
                    }
                    _ => panic!("Expected JsonTypeDef validation failures"),
                }
                assert_eq!(validated_data, Box::new(json!({ "foo": "bar" })));
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }

    #[slipway_test_async]
    async fn it_should_validate_component_input_with_json_schema() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(
                            "b.input",
                            json!({
                                "$schema": "http://json-schema.org/draft-07/schema#",
                                "properties": {
                                    "a_output": {
                                        "properties": {
                                            "foo": {
                                                "type": "string"
                                            }
                                        },
                                        "required": ["foo"],
                                        "additionalProperties": false
                                    },
                                },
                                "required": ["a_output"],
                                "additionalProperties": false
                            }),
                        )
                        .await,
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let mut s = rig_session.initialize().unwrap();

        s = s
            .step(Instruction::SetOutput {
                handle: ch("a"),
                value: json!({ "foo": "bar" }),
                metadata: Default::default(),
            })
            .unwrap();

        let b_component_state = s.get_component_state(&ch("b")).unwrap();
        let b_execution_input = b_component_state.execution_input.as_ref().unwrap();
        assert_eq!(
            b_execution_input.value,
            json!({ "a_output": { "foo": "bar" } })
        );
    }

    #[slipway_test_async]
    async fn it_should_fail_to_validate_invalid_component_input_with_json_schema() {
        let rig = create_rig();

        let component_cache = BasicComponentCache::for_test_with_schemas(
            &rig,
            [
                ("a".to_string(), (schema_any(), schema_any())),
                (
                    "b".to_string(),
                    (
                        schema_valid(
                            "b.input",
                            json!({
                                "$schema": "http://json-schema.org/draft-07/schema#",
                                "properties": {
                                    "a_output": {
                                        "properties": {
                                            "foo": {
                                                "type": "number"
                                            }
                                        },
                                        "required": ["foo"],
                                        "additionalProperties": false
                                    },
                                },
                                "required": ["a_output"],
                                "additionalProperties": false
                            }),
                        )
                        .await,
                        schema_any(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .await;

        let rig_session = RigSession::new_for_test(rig, &component_cache);

        let s = rig_session.initialize().unwrap();
        let s_result = s.step(Instruction::SetOutput {
            handle: ch("a"),
            value: json!({ "foo": "bar" }),
            metadata: Default::default(),
        });

        match s_result {
            Err(RigError::ComponentValidationFailed {
                component_handle,
                validation_type,
                validation_failures,
                validated_data,
            }) => {
                assert_eq!(component_handle, ch("b"));
                assert_eq!(validation_type, ValidationType::Input);

                match validation_failures {
                    SchemaValidationFailures::JsonSchema(validation_failures) => {
                        assert_eq!(validation_failures.len(), 1);
                        assert_eq!(validation_failures[0].instance_path(), "/a_output/foo");
                        assert_eq!(
                            validation_failures[0].schema_path(),
                            "/properties/a_output/properties/foo/type"
                        );
                    }
                    _ => panic!("Expected JsonTypeDef validation failures"),
                }

                assert_eq!(
                    validated_data,
                    Box::new(json!({ "a_output": { "foo": "bar" } }))
                );
            }
            _ => panic!("Expected ComponentValidationFailed error"),
        }
    }
}
