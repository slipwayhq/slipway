use std::collections::HashMap;

use super::parse::{Component, ComponentInput, ComponentInputSpecification};

pub(crate) enum ValidationFailure {
    Error(String),
    Warning(String),
}

pub(crate) fn validate_component(component: &Component) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    failures.append(&mut validate_inputs(&component.inputs));

    failures
}

fn validate_inputs(inputs: &[ComponentInput]) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    for (index, input) in inputs.iter().enumerate() {
        if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
            failures.push(ValidationFailure::Error(format!(
                "Duplicate input id: {}",
                input.id
            )));
        }

        let input_name = input.get_name();
        if inputs
            .iter()
            .skip(index + 1)
            .any(|i| i.get_name() == input_name)
        {
            failures.push(ValidationFailure::Warning(format!(
                "Duplicate input name: {}",
                input_name
            )));
        }

        failures.append(&mut validate_input(input));
    }

    failures
}

fn validate_input(input: &ComponentInput) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    if input.default_component.is_some() && input.default_value.is_some() {
        failures.push(ValidationFailure::Error(format!(
            "Input {} has both a default component and a default value",
            input.id
        )));
    }

    if let Some(default_component) = &input.default_component {
        failures.append(&mut validate_component_input_specification(
            default_component,
        ));
    }

    failures
}

fn validate_component_input_specification(
    default_component: &ComponentInputSpecification,
) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    if let Some(inputs) = &default_component.inputs {
        for (index, input) in inputs.iter().enumerate() {
            if inputs.iter().skip(index + 1).any(|i| i.id == input.id) {
                failures.push(ValidationFailure::Error(format!(
                    "Duplicate override input id: {}",
                    input.id
                )));
            }

            failures.append(&mut validate_input_override(input));
        }
    }

    failures
}

fn validate_input_override(input: &super::parse::ComponentInputOverride) -> Vec<ValidationFailure> {
    let mut failures = vec![];

    if let Some(component) = &input.component {
        failures.append(&mut validate_component_input_specification(component));
    }

    failures
}

#[cfg(test)]
mod tests {
    use crate::rigging::parse::{ComponentOutput, ComponentReference};

    use super::*;

    #[test]
    fn when_component_is_valid_it_should_not_throw() {
        let component = Component {
            id: "test".to_string(),
            description: Some("Test component".to_string()),
            version: "1.0.0".to_string(),
            inputs: vec![],
            output: ComponentOutput {
                schema_reference: Some("output_schema:1.0".parse::<ComponentReference>().unwrap()),
                schema: None,
            },
        };

        validate_component(&component);
    }
}
