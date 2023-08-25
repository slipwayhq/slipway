use crate::rigging::parse::types::{Component, ComponentInputSpecification, ComponentReference};

pub(crate) fn find_component_references(component: &Component) -> Vec<ComponentReference> {
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
    references: &mut Vec<ComponentReference>,
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

fn add_if_not_exists(references: &mut Vec<ComponentReference>, reference: &ComponentReference) {
    if !references.contains(reference) {
        references.push(reference.clone());
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use crate::rigging::parse::types::{ComponentInput, ComponentInputOverride, ComponentOutput};

    use super::*;

    #[test]
    fn it_should_find_all_component_references() {
        let component = Component {
            id: "component".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            inputs: vec![
                ComponentInput {
                    id: "input1".to_string(),
                    name: None,
                    description: None,
                    schema: None,
                    default_component: Some(ComponentInputSpecification {
                        reference: ComponentReference {
                            id: "component1".to_string(),
                            version: "1.0".to_string(),
                        },
                        input_overrides: None,
                    }),
                    default_value: None,
                },
                ComponentInput {
                    id: "input2".to_string(),
                    name: None,
                    description: None,
                    schema: None,
                    default_component: Some(ComponentInputSpecification {
                        reference: ComponentReference {
                            id: "component2".to_string(),
                            version: "1.0".to_string(),
                        },
                        input_overrides: Some(vec![
                            ComponentInputOverride {
                                id: "input3".to_string(),
                                component: Some(ComponentInputSpecification {
                                    reference: ComponentReference {
                                        id: "component3".to_string(),
                                        version: "1.0".to_string(),
                                    },
                                    input_overrides: Some(vec![ComponentInputOverride {
                                        id: "input4".to_string(),
                                        component: Some(ComponentInputSpecification {
                                            reference: ComponentReference {
                                                id: "component2".to_string(),
                                                version: "1.0".to_string(),
                                            },
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
                                    reference: ComponentReference {
                                        id: "component2".to_string(),
                                        version: "1.1".to_string(),
                                    },
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
                schema_reference: Some(ComponentReference {
                    id: "component4".to_string(),
                    version: "1.0".to_string(),
                }),
            },
        };

        let mut references = find_component_references(&component);

        references.sort_by(|a, b| {
            let id_cmp = a.id.cmp(&b.id);
            if id_cmp == Ordering::Equal {
                a.version.cmp(&b.version)
            } else {
                id_cmp
            }
        });

        assert_eq!(
            references,
            vec![
                ComponentReference {
                    id: "component1".to_string(),
                    version: "1.0".to_string(),
                },
                ComponentReference {
                    id: "component2".to_string(),
                    version: "1.0".to_string(),
                },
                ComponentReference {
                    id: "component2".to_string(),
                    version: "1.1".to_string(),
                },
                ComponentReference {
                    id: "component3".to_string(),
                    version: "1.0".to_string(),
                },
                ComponentReference {
                    id: "component4".to_string(),
                    version: "1.0".to_string(),
                },
            ]
        );
    }
}
