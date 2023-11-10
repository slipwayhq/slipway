pub mod context;
pub mod validation_failure;

use std::rc::Rc;

use self::{context::Context, validation_failure::ValidationFailure};

use super::parse::types::{
    Component, ComponentInput, ComponentInputOverride, ComponentInputSpecification,
    ResolvedComponentReference, UnresolvedComponentReference,
};

pub struct ValidationResult {
    pub reference: ResolvedComponentReference,
    pub failures: Vec<ValidationFailure>,
}

const ROOT_REFERENCE_NOT_ALLOWED: &str = "Root component is not a valid reference";

#[must_use]
pub fn validate_component(
    expected: &UnresolvedComponentReference,
    component: &Component,
) -> ValidationResult {
    let mut failures = vec![];

    let context = Rc::new(Context {
        node_name: component.name.clone(),
        previous_context: None,
    });

    check_component_matches_unresolved_reference(&mut failures, expected, component, &context);

    let inputs_context = Rc::new(Context {
        node_name: "inputs".to_string(),
        previous_context: Some(Rc::clone(&context)),
    });

    validate_inputs(&mut failures, &component.inputs, inputs_context);

    let output_context = Rc::new(Context {
        node_name: "output".to_string(),
        previous_context: Some(Rc::clone(&context)),
    });
    validate_reference_option(
        &mut failures,
        &component.output.schema_reference,
        &output_context,
    );

    ValidationResult {
        reference: component.get_reference(),
        failures,
    }
}

fn check_component_matches_unresolved_reference(
    failures: &mut Vec<ValidationFailure>,
    expected: &UnresolvedComponentReference,
    component: &Component,
    context: &Rc<Context>,
) {
    // We check the unresolved reference to ensure this is the component we were expecting.
    match expected {
        UnresolvedComponentReference::Registry {
            publisher,
            name,
            version: _,
        } => {
            if publisher != &component.publisher || name != &component.name {
                failures.push(ValidationFailure::Error(
                    format!(
                        r#"Expected component ID "{}.{}" but found "{}.{}""#,
                        publisher, name, component.publisher, component.name
                    ),
                    Rc::clone(context),
                ));
            }
        }

        UnresolvedComponentReference::GitHub {
            user,
            repository,
            version: _,
        } => {
            if user != &component.publisher || repository != &component.name {
                failures.push(ValidationFailure::Warning(
                    format!(
                        r#"Expected component ID "{}.{}" but found "{}.{}""#,
                        user, repository, component.publisher, component.name
                    ),
                    Rc::clone(context),
                ));
            }
        }

        UnresolvedComponentReference::Url { url } => {
            if !url.as_str().contains(&component.name) {
                failures.push(ValidationFailure::Warning(
                    format!(
                        r#"URL reference "{}" resolved to component with name "{}" but the name was not present in the URL"#,
                        url, component.name
                    ),
                    Rc::clone(context),
                ));
            }
        }

        UnresolvedComponentReference::Local { path } => {
            let lossy_path = path.to_string_lossy();
            if !lossy_path.contains(&component.name) {
                failures.push(ValidationFailure::Warning(
                    format!(
                        r#"Local reference "{}" resolved to component with name "{}" but the name was not present in the path"#,
                        lossy_path, component.name
                    ),
                    Rc::clone(context),
                ));
            }
        }

        UnresolvedComponentReference::Root => {}
    }
}

fn validate_reference_option(
    failures: &mut Vec<ValidationFailure>,
    reference: &Option<UnresolvedComponentReference>,
    context: &Rc<Context>,
) {
    if let Some(reference) = reference {
        validate_reference(failures, reference, context);
    }
}

fn validate_reference(
    failures: &mut Vec<ValidationFailure>,
    reference: &UnresolvedComponentReference,
    context: &Rc<Context>,
) {
    // Referencing root is guaranteed to be a circular reference, and is forbidden.
    if reference == &UnresolvedComponentReference::Root {
        failures.push(ValidationFailure::Error(
            ROOT_REFERENCE_NOT_ALLOWED.to_string(),
            Rc::clone(context),
        ));
    }
}

fn validate_inputs(
    failures: &mut Vec<ValidationFailure>,
    inputs: &[ComponentInput],
    context: Rc<Context>,
) {
    for (index, input) in inputs.iter().enumerate() {
        if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
            failures.push(ValidationFailure::Error(
                format!("Duplicate input id: {}", input.id),
                Rc::clone(&context),
            ));
        }

        let input_name = input.get_display_name();
        if inputs
            .iter()
            .skip(index + 1)
            .any(|i| i.get_display_name() == input_name)
        {
            failures.push(ValidationFailure::Warning(
                format!("Duplicate input name: {}", input_name),
                Rc::clone(&context),
            ));
        }

        let input_context = Rc::new(Context {
            node_name: input.id.clone(),
            previous_context: Some(Rc::clone(&context)),
        });

        validate_input(failures, input, input_context);
    }
}

fn validate_input(
    failures: &mut Vec<ValidationFailure>,
    input: &ComponentInput,
    context: Rc<Context>,
) {
    if input.default_component.is_some() && input.default_value.is_some() {
        failures.push(ValidationFailure::Error(
            format!(
                r#"Input "{}" has both a default component and a default value"#,
                input.id
            ),
            Rc::clone(&context),
        ));
    }

    if let Some(default_component) = &input.default_component {
        let default_component_context = Rc::new(Context {
            node_name: "default_component".to_string(),
            previous_context: Some(Rc::clone(&context)),
        });

        validate_component_input_specification(
            failures,
            default_component,
            default_component_context,
        );
    }
}

fn validate_component_input_specification(
    failures: &mut Vec<ValidationFailure>,
    default_component: &ComponentInputSpecification,
    context: Rc<Context>,
) {
    validate_reference(failures, &default_component.reference, &context);

    let input_overrides_context = Rc::new(Context {
        node_name: "input_overrides".to_string(),
        previous_context: Some(Rc::clone(&context)),
    });

    if let Some(inputs) = &default_component.input_overrides {
        for (index, input) in inputs.iter().enumerate() {
            if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
                failures.push(ValidationFailure::Error(
                    format!("Duplicate input override id: {}", input.id),
                    Rc::clone(&input_overrides_context),
                ));
            }

            let input_context = Rc::new(Context {
                node_name: input.id.clone(),
                previous_context: Some(Rc::clone(&input_overrides_context)),
            });

            validate_input_override(failures, input, input_context);
        }
    }
}

fn validate_input_override(
    failures: &mut Vec<ValidationFailure>,
    input: &ComponentInputOverride,
    context: Rc<Context>,
) {
    if input.component.is_some() && input.value.is_some() {
        failures.push(ValidationFailure::Error(
            format!(
                r#"Input override "{}" has both a component and a value"#,
                input.id
            ),
            Rc::clone(&context),
        ));
    }

    if let Some(component) = &input.component {
        let component_context = Rc::new(Context {
            node_name: "component".to_string(),
            previous_context: Some(Rc::clone(&context)),
        });

        validate_component_input_specification(failures, component, component_context);
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;
    use serde_json::json;

    use crate::rigging::parse::types::{
        ComponentInputOverride, ComponentOutput, UnresolvedComponentReference, TEST_PUBLISHER,
    };

    use super::*;

    #[test]
    fn when_component_is_valid_it_should_return_no_failures() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(
            &UnresolvedComponentReference::test("test", "1.1"),
            &component,
        );
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 0);
    }

    #[test]
    fn when_component_has_unexpected_name_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(
            &UnresolvedComponentReference::test("test2", "1.1"),
            &component,
        );
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Expected component ID "test-publisher.test2" but found "test-publisher.test" (test)"#
        );
    }

    #[test]
    fn when_component_has_unexpected_publisher_it_should_return_error() {
        let component = Component {
            publisher: "other-publisher".to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(
            &UnresolvedComponentReference::test("test", "1.1"),
            &component,
        );
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Expected component ID "test-publisher.test" but found "other-publisher.test" (test)"#
        );
    }

    #[test]
    fn when_no_expected_id_it_should_no_failures() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 0);
    }

    #[test]
    fn when_component_is_has_duplicate_input_ids_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![
                ComponentInput {
                    id: "input-one".to_string(),
                    display_name: Some("Input 1".to_string()),
                    description: Some("Input 1 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
                ComponentInput {
                    id: "input-one".to_string(),
                    display_name: Some("Input 2".to_string()),
                    description: Some("Input 2 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
            ],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            "Error: Duplicate input id: input-one (test.inputs)"
        );
    }

    #[test]
    fn when_component_is_has_duplicate_input_names_it_should_return_warning() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![
                ComponentInput {
                    id: "input-one".to_string(),
                    display_name: Some("Input 1".to_string()),
                    description: Some("Input 1 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
                ComponentInput {
                    id: "input-two".to_string(),
                    display_name: Some("Input 1".to_string()),
                    description: Some("Input 2 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
            ],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            "Warning: Duplicate input name: Input 1 (test.inputs)"
        );
    }

    #[test]
    fn when_component_is_has_both_default_component_and_value_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::test("default_component", "1.0"),
                    input_overrides: None,
                }),
                default_value: Some(json!(3)),
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Input "input-one" has both a default component and a default value (test.inputs.input-one)"#
        );
    }

    #[test]
    fn when_component_references_root_component_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: None,
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::Root),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            format!("Error: {} (test.output)", ROOT_REFERENCE_NOT_ALLOWED)
        );
    }

    #[test]
    fn when_component_override_references_root_component_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::Root,
                    input_overrides: None,
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            format!(
                r#"Error: {} (test.inputs.input-one.default_component)"#,
                ROOT_REFERENCE_NOT_ALLOWED
            )
        );
    }

    #[test]
    fn when_component_input_override_references_root_component_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::test("default_component", "1.0"),
                    input_overrides: Some(vec![ComponentInputOverride {
                        id: "sub-input-one".to_string(),
                        component: Some(ComponentInputSpecification {
                            reference: UnresolvedComponentReference::Root,
                            input_overrides: None,
                        }),
                        value: None,
                    }]),
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            format!(
                r#"Error: {} (test.inputs.input-one.default_component.input_overrides.sub-input-one.component)"#,
                ROOT_REFERENCE_NOT_ALLOWED
            )
        );
    }

    #[test]
    fn when_component_override_has_duplicate_input_id_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::test("default_component", "1.0"),
                    input_overrides: Some(vec![
                        ComponentInputOverride {
                            id: "sub-input-one".to_string(),
                            component: None,
                            value: Some(json!(3)),
                        },
                        ComponentInputOverride {
                            id: "sub-input-one".to_string(),
                            component: None,
                            value: Some(json!(4)),
                        },
                    ]),
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Duplicate input override id: sub-input-one (test.inputs.input-one.default_component.input_overrides)"#
        );
    }

    #[test]
    fn when_component_override_has_deep_duplicate_input_id_it_should_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::test("default_component", "1.0"),
                    input_overrides: Some(vec![ComponentInputOverride {
                        id: "sub-input-one".to_string(),
                        component: Some(ComponentInputSpecification {
                            reference: UnresolvedComponentReference::test(
                                "default_component_2",
                                "1.0",
                            ),
                            input_overrides: Some(vec![
                                ComponentInputOverride {
                                    id: "sub-sub-input-one".to_string(),
                                    component: None,
                                    value: Some(json!(3)),
                                },
                                ComponentInputOverride {
                                    id: "sub-sub-input-one".to_string(),
                                    component: None,
                                    value: Some(json!(4)),
                                },
                            ]),
                        }),
                        value: None,
                    }]),
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Duplicate input override id: sub-sub-input-one (test.inputs.input-one.default_component.input_overrides.sub-input-one.component.input_overrides)"#
        );
    }

    #[test]
    fn when_component_input_override_has_both_component_and_value_return_error() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "test".to_string(),
            description: Some("Test component".to_string()),
            version: Version::new(1, 0, 0),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                display_name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: UnresolvedComponentReference::test("default_component", "1.0"),
                    input_overrides: Some(vec![ComponentInputOverride {
                        id: "sub-input-one".to_string(),
                        component: Some(ComponentInputSpecification {
                            reference: UnresolvedComponentReference::test(
                                "default_component_2",
                                "1.0",
                            ),
                            input_overrides: None,
                        }),
                        value: Some(json!(3)),
                    }]),
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(UnresolvedComponentReference::test("output_schema", "1.0")),
                schema: None,
            },
        };

        let validate_result = validate_component(&UnresolvedComponentReference::Root, &component);
        let failures = validate_result.failures;

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Input override "sub-input-one" has both a component and a value (test.inputs.input-one.default_component.input_overrides.sub-input-one)"#
        );
    }
}
