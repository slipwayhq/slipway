pub mod validation_context;
pub mod validation_failure;

use std::rc::Rc;

use self::{validation_context::ValidationContext, validation_failure::ValidationFailure};

use super::parse::types::{
    Component, ComponentInput, ComponentInputOverride, ComponentInputSpecification,
};

#[must_use]
pub fn validate_component(component: &Component) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    let context = Rc::new(ValidationContext {
        node_name: component.id.clone(),
        previous_context: None,
    });

    let inputs_context = Rc::new(ValidationContext {
        node_name: "inputs".to_string(),
        previous_context: Some(Rc::clone(&context)),
    });

    failures.append(&mut validate_inputs(&component.inputs, inputs_context));

    failures
}

#[must_use]
fn validate_inputs(
    inputs: &[ComponentInput],
    context: Rc<ValidationContext>,
) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    for (index, input) in inputs.iter().enumerate() {
        if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
            failures.push(ValidationFailure::Error(
                format!("Duplicate input id: {}", input.id),
                Rc::clone(&context),
            ));
        }

        let input_name = input.get_name();
        if inputs
            .iter()
            .skip(index + 1)
            .any(|i| i.get_name() == input_name)
        {
            failures.push(ValidationFailure::Warning(
                format!("Duplicate input name: {}", input_name),
                Rc::clone(&context),
            ));
        }

        let input_context = Rc::new(ValidationContext {
            node_name: input.id.clone(),
            previous_context: Some(Rc::clone(&context)),
        });

        failures.append(&mut validate_input(input, input_context));
    }

    failures
}

#[must_use]
fn validate_input(
    input: &ComponentInput,
    context: Rc<ValidationContext>,
) -> Vec<ValidationFailure> {
    let mut failures = vec![];

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
        let default_component_context = Rc::new(ValidationContext {
            node_name: "default_component".to_string(),
            previous_context: Some(Rc::clone(&context)),
        });

        failures.append(&mut validate_component_input_specification(
            default_component,
            default_component_context,
        ));
    }

    failures
}

#[must_use]
fn validate_component_input_specification(
    default_component: &ComponentInputSpecification,
    context: Rc<ValidationContext>,
) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    let inputs_context = Rc::new(ValidationContext {
        node_name: "inputs".to_string(),
        previous_context: Some(Rc::clone(&context)),
    });

    if let Some(inputs) = &default_component.inputs {
        for (index, input) in inputs.iter().enumerate() {
            if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
                failures.push(ValidationFailure::Error(
                    format!("Duplicate override input id: {}", input.id),
                    Rc::clone(&inputs_context),
                ));
            }

            let input_context = Rc::new(ValidationContext {
                node_name: input.id.clone(),
                previous_context: Some(Rc::clone(&inputs_context)),
            });

            failures.append(&mut validate_input_override(input, input_context));
        }
    }

    failures
}

#[must_use]
fn validate_input_override(
    input: &ComponentInputOverride,
    context: Rc<ValidationContext>,
) -> Vec<ValidationFailure> {
    let mut failures = vec![];

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
        let component_context = Rc::new(ValidationContext {
            node_name: "component".to_string(),
            previous_context: Some(Rc::clone(&context)),
        });

        failures.append(&mut validate_component_input_specification(
            component,
            component_context,
        ));
    }

    failures
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::rigging::parse::types::{
        ComponentInputOverride, ComponentOutput, ComponentReference,
    };

    use super::*;

    #[test]
    fn when_component_is_valid_it_should_return_no_failures() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 0);
    }

    #[test]
    fn when_component_is_has_duplicate_input_ids_it_should_return_error() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![
                ComponentInput {
                    id: "input-one".to_string(),
                    name: Some("Input 1".to_string()),
                    description: Some("Input 1 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
                ComponentInput {
                    id: "input-one".to_string(),
                    name: Some("Input 2".to_string()),
                    description: Some("Input 2 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
            ],
            output: ComponentOutput {
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            "Error: Duplicate input id: input-one (test.inputs)"
        );
    }

    #[test]
    fn when_component_is_has_duplicate_input_names_it_should_return_warning() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![
                ComponentInput {
                    id: "input-one".to_string(),
                    name: Some("Input 1".to_string()),
                    description: Some("Input 1 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
                ComponentInput {
                    id: "input-two".to_string(),
                    name: Some("Input 1".to_string()),
                    description: Some("Input 2 description".to_string()),
                    schema: None,
                    default_component: None,
                    default_value: None,
                },
            ],
            output: ComponentOutput {
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            "Warning: Duplicate input name: Input 1 (test.inputs)"
        );
    }

    #[test]
    fn when_component_is_has_both_default_component_and_value_it_should_return_error() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: ComponentReference {
                        id: "default_component".to_string(),
                        version: "1.0".to_string(),
                    },
                    inputs: None,
                }),
                default_value: Some(json!(3)),
            }],
            output: ComponentOutput {
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Input "input-one" has both a default component and a default value (test.inputs.input-one)"#
        );
    }

    #[test]
    fn when_component_override_has_duplicate_input_id_it_should_return_error() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: ComponentReference {
                        id: "default_component".to_string(),
                        version: "1.0".to_string(),
                    },
                    inputs: Some(vec![
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
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Duplicate override input id: sub-input-one (test.inputs.input-one.default_component.inputs)"#
        );
    }

    #[test]
    fn when_component_override_has_deep_duplicate_input_id_it_should_return_error() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: ComponentReference {
                        id: "default_component".to_string(),
                        version: "1.0".to_string(),
                    },
                    inputs: Some(vec![ComponentInputOverride {
                        id: "sub-input-one".to_string(),
                        component: Some(ComponentInputSpecification {
                            reference: ComponentReference {
                                id: "default_component_2".to_string(),
                                version: "1.0".to_string(),
                            },
                            inputs: Some(vec![
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
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Duplicate override input id: sub-sub-input-one (test.inputs.input-one.default_component.inputs.sub-input-one.component.inputs)"#
        );
    }

    #[test]
    fn when_component_input_override_has_both_component_and_value_return_error() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![ComponentInput {
                id: "input-one".to_string(),
                name: Some("Input 1".to_string()),
                description: Some("Input 1 description".to_string()),
                schema: None,
                default_component: Some(ComponentInputSpecification {
                    reference: ComponentReference {
                        id: "default_component".to_string(),
                        version: "1.0".to_string(),
                    },
                    inputs: Some(vec![ComponentInputOverride {
                        id: "sub-input-one".to_string(),
                        component: Some(ComponentInputSpecification {
                            reference: ComponentReference {
                                id: "default_component_2".to_string(),
                                version: "1.0".to_string(),
                            },
                            inputs: None,
                        }),
                        value: Some(json!(3)),
                    }]),
                }),
                default_value: None,
            }],
            output: ComponentOutput {
                schema_reference: Some(ComponentReference {
                    id: "output_schema".to_string(),
                    version: "1.0".to_string(),
                }),
                schema: None,
            },
        };

        let failures = validate_component(&component);

        assert_eq!(failures.len(), 1);

        assert_eq!(
            failures[0].to_string(),
            r#"Error: Input override "sub-input-one" has both a component and a value (test.inputs.input-one.default_component.inputs.sub-input-one)"#
        );
    }
}
