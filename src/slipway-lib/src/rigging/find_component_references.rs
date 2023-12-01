use crate::rigging::parse::types::{
    Component, ComponentInputSpecification, UnresolvedComponentReference,
};

pub(crate) fn find_component_references(
    component: &Component,
) -> Vec<UnresolvedComponentReference> {
    let mut references = Vec::new();

    for input in &component.inputs {
        if let Some(default_component) = &input.default_component {
            find_in_input_specification(&mut references, default_component);
        }
    }

    if let Some(schema_reference) = &component.output.schema_reference {
        add_if_not_exists(&mut references, schema_reference);
    }

    references
}

fn find_in_input_specification(
    references: &mut Vec<UnresolvedComponentReference>,
    default_component: &ComponentInputSpecification,
) {
    add_if_not_exists(references, &default_component.reference);

    if let Some(input_overrides) = &default_component.input_overrides {
        for input_override in input_overrides {
            if let Some(component) = &input_override.component {
                find_in_input_specification(references, component);
            }
        }
    }
}

fn add_if_not_exists(
    references: &mut Vec<UnresolvedComponentReference>,
    reference: &UnresolvedComponentReference,
) {
    if !references.contains(reference) {
        references.push(reference.clone());
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use crate::rigging::parse::types::{
        ComponentInput, ComponentInputOverride, ComponentOutput, TEST_PUBLISHER,
    };

    use super::*;

    #[test]
    fn it_should_find_all_component_references() {
        let component = Component {
            publisher: TEST_PUBLISHER.to_string(),
            name: "component".to_string(),
            description: None,
            version: Version::new(1, 0, 0),
            inputs: vec![
                ComponentInput {
                    id: "input1".to_string(),
                    display_name: None,
                    description: None,
                    schema: None,
                    default_component: Some(ComponentInputSpecification {
                        reference: UnresolvedComponentReference::for_test("component1", "1.0"),
                        input_overrides: None,
                    }),
                    default_value: None,
                },
                ComponentInput {
                    id: "input2".to_string(),
                    display_name: None,
                    description: None,
                    schema: None,
                    default_component: Some(ComponentInputSpecification {
                        reference: UnresolvedComponentReference::for_test("component2", "1.0"),
                        input_overrides: Some(vec![
                            ComponentInputOverride {
                                id: "input3".to_string(),
                                component: Some(ComponentInputSpecification {
                                    reference: UnresolvedComponentReference::for_test(
                                        "component3",
                                        "1.0",
                                    ),
                                    input_overrides: Some(vec![ComponentInputOverride {
                                        id: "input4".to_string(),
                                        component: Some(ComponentInputSpecification {
                                            reference: UnresolvedComponentReference::for_test(
                                                "component2",
                                                "1.0",
                                            ),
                                            input_overrides: None,
                                        }),
                                        value: None,
                                    }]),
                                }),
                                value: None,
                            },
                            ComponentInputOverride {
                                id: "input5".to_string(),
                                component: Some(ComponentInputSpecification {
                                    reference: UnresolvedComponentReference::for_test(
                                        "component2",
                                        "1.1",
                                    ),
                                    input_overrides: None,
                                }),
                                value: None,
                            },
                        ]),
                    }),
                    default_value: None,
                },
            ],
            output: ComponentOutput {
                schema: None,
                schema_reference: Some(UnresolvedComponentReference::for_test("component4", "1.0")),
            },
        };

        let mut references = find_component_references(&component);

        references.sort_by_key(|a| a.to_string());

        assert_eq!(
            references,
            vec![
                UnresolvedComponentReference::for_test("component1", "1.0"),
                UnresolvedComponentReference::for_test("component2", "1.0"),
                UnresolvedComponentReference::for_test("component2", "1.1"),
                UnresolvedComponentReference::for_test("component3", "1.0"),
                UnresolvedComponentReference::for_test("component4", "1.0"),
            ]
        );
    }
}
