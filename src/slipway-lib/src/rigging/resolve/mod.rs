mod build_context;

use std::{collections::HashMap, sync::OnceLock};

use async_executor::LocalExecutor;
use async_trait::async_trait;
use futures_lite::{future, FutureExt};
use thiserror::Error;
use typed_arena::Arena;

use crate::errors::SlipwayError;
use crate::rigging::find_component_references::find_component_references;

pub(crate) use build_context::BuildContext;

use self::build_context::BuildContextSnapshot;

use super::{
    parse::{
        parse_component,
        types::{Component, ComponentReference},
    },
    validate::{validate_component, validation_failure::ValidationFailure},
};

pub(crate) fn resolve_components(
    root_component: String,
    component_reference_resolver: Box<dyn ComponentReferenceResolver>,
) -> ResolvedComponents {
    let local_executor = LocalExecutor::new();

    future::block_on(local_executor.run(resolve_components_async(
        root_component,
        component_reference_resolver,
    )))
}

pub(crate) struct ResolvedComponents {
    resolved: HashMap<ComponentReference, Component>,
    failed: HashMap<ComponentReference, BuildComponentFailure>,
    root_warnings: Vec<ValidationFailure>,
}

async fn resolve_components_async(
    root_component: String,
    component_reference_resolver: Box<dyn ComponentReferenceResolver>,
) -> ResolvedComponents {
    // We're going to create all the contexts in an arena, so they can hold
    // references to each other and all have the same lifetime.
    let context_arena = Arena::new();

    let root_context = context_arena.alloc(BuildContext {
        reference: ComponentReference::root(),
        resolved_reference: OnceLock::new(),
        previous_context: None,
    });

    // This is a list of component references which are in the process of being
    // fetched, or possibly have completed being fetched and are waiting to be
    // processed. We add the root component to the list as the first item to process.
    let mut futures = vec![future::ready(Result::<
        ResolvedReferenceContent,
        ComponentReferenceResolveError,
    >::Ok(ResolvedReferenceContent {
        context: root_context,
        rigging: root_component,
    }))
    .boxed()];

    let mut validated = HashMap::new();
    let mut failed = HashMap::new();
    let mut root_warnings = Vec::new();

    // When any task which has not been processed is ready, process it.
    while !futures.is_empty() {
        let (result, _, remaining_futures) = futures_util::future::select_all(futures).await;
        futures = remaining_futures;

        match result {
            Err(e) => {
                failed.insert(
                    e.context.reference.clone(),
                    BuildComponentFailure::Resolve {
                        source: e.source,
                        context: e.context.as_list(),
                    },
                );
            }
            Ok(result) => {
                let context = result.context;
                let context_component_reference = context.reference.clone();
                let is_root = context_component_reference.is_root();

                let parse_result = parse_component(&result.rigging);

                match parse_result {
                    Err(e) => {
                        failed.insert(
                            context_component_reference,
                            BuildComponentFailure::Parse {
                                source: e,
                                context: context.as_list(),
                            },
                        );
                    }
                    Ok(component) => {
                        let expected_component_id = match is_root {
                            true => None,
                            false => Some(context_component_reference.id.clone()),
                        };

                        let validation_result =
                            validate_component(expected_component_id, &component);

                        let (warnings, errors) = validation_result
                            .failures
                            .into_iter()
                            .partition(|f| matches!(f, ValidationFailure::Warning(_, _)));

                        if is_root {
                            root_warnings = warnings;
                        }

                        if !errors.is_empty() {
                            failed.insert(
                                context_component_reference.clone(),
                                BuildComponentFailure::Validate {
                                    validation_errors: errors,
                                    context: context.as_list(),
                                },
                            );
                        } else {
                            let references = find_component_references(&component);

                            // Set the resolved reference.
                            context
                                .resolved_reference
                                .set(component.get_reference())
                                .unwrap_or_else(|v| {
                                    panic!(
                                        r#"Resolved component reference "{v}" should only be set once"#,
                                    )
                                });

                            let circular_reference = references
                                .iter()
                                .find(|reference| context.contains_resolved_id(&reference.id));

                            if let Some(circular_reference) = circular_reference {
                                failed.insert(
                                    context_component_reference.clone(),
                                    BuildComponentFailure::CircularReference {
                                        reference: circular_reference.clone(),
                                        context: context.as_list(),
                                    },
                                );
                            } else {
                                validated.insert(context_component_reference.clone(), component);

                                for reference in references {
                                    if validated.contains_key(&reference)
                                        || failed.contains_key(&reference)
                                    {
                                        break;
                                    }

                                    let new_context = context_arena.alloc(BuildContext {
                                        reference: reference.clone(),
                                        resolved_reference: OnceLock::new(),
                                        previous_context: Some(context),
                                    });

                                    futures.push(
                                        component_reference_resolver
                                            .resolve(reference, new_context),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ResolvedComponents {
        resolved: validated,
        failed,
        root_warnings,
    }
}

#[derive(Debug)]
enum BuildComponentFailure {
    Resolve {
        source: SlipwayError,
        context: Vec<BuildContextSnapshot>,
    },
    Parse {
        source: SlipwayError,
        context: Vec<BuildContextSnapshot>,
    },
    Validate {
        validation_errors: Vec<ValidationFailure>,
        context: Vec<BuildContextSnapshot>,
    },
    CircularReference {
        reference: ComponentReference,
        context: Vec<BuildContextSnapshot>,
    },
}

#[async_trait]
pub(crate) trait ComponentReferenceResolver {
    async fn resolve<'a, 'b>(
        &self,
        reference: ComponentReference,
        context: &'a BuildContext<'a>,
    ) -> Result<ResolvedReferenceContent<'b>, ComponentReferenceResolveError<'b>>
    where
        'a: 'b;
}

pub(crate) struct ResolvedReferenceContent<'a> {
    pub context: &'a BuildContext<'a>,
    pub rigging: String,
}

#[derive(Error, Debug)]
#[error("Rigging parse failed")]
pub(crate) struct ComponentReferenceResolveError<'a> {
    pub context: &'a BuildContext<'a>,
    pub source: SlipwayError,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print_failures(resolved_components: &ResolvedComponents) {
        resolved_components
            .failed
            .iter()
            .for_each(|e| print!("{:?}", e));
    }

    #[test]
    fn it_should_resolve_all_references() {
        let root_component = r#"
        {
            "id": "test",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": {
                            "id": "test2",
                            "version": "1.0.0"
                        }
                    }
                }
            ],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test2_v1_component = r#"
        {
            "id": "test2",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": {
                            "id": "test3",
                            "version": "1.0.0"
                        },
                        "input_overrides": [
                            {
                                "id": "input1",
                                "component": {
                                    "reference": {
                                        "id": "test4",
                                        "version": "1.0.0"
                                    }
                                }
                            }
                        ]
                    }
                }
            ],
            "output": {
                "schema_reference": "test3@2.0.0"
            }
        }"#;

        let test3_v1_component = r#"
        {
            "id": "test3",
            "version": "1.0.0",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test3_v2_component = r#"
        {
            "id": "test3",
            "version": "2.0.0",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test4_v1_component = r#"
        {
            "id": "test4",
            "version": "1.0.0",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let resolver = MockComponentReferenceResolver {
            resolved: vec![
                (
                    ComponentReference::exact("test2", "1.0.0"),
                    test2_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "1.0.0"),
                    test3_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "2.0.0"),
                    test3_v2_component.to_string(),
                ),
                (
                    ComponentReference::exact("test4", "1.0.0"),
                    test4_v1_component.to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        };

        let resolved_components =
            resolve_components(root_component.to_string(), Box::new(resolver));

        print_failures(&resolved_components);

        assert!(resolved_components.failed.is_empty());
        assert_eq!(resolved_components.resolved.len(), 5);
    }

    #[test]
    fn it_should_return_errors_and_warnings_for_root_component_but_only_errors_for_referenced_components(
    ) {
        // This contains a duplicate name (warning) and
        // a duplicate id (error).
        let test1_component = r#"
        {
            "id": "test1",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input One",
                    "default_value": 1
                },
                {
                    "id": "input2",
                    "name": "Input One",
                    "default_value": 2
                },
                {
                    "id": "input2",
                    "default_value": 3
                }
            ],
            "output": {
                "schema_reference": "foo@1.0"
            }
        }"#;

        // This just references test1.
        let test2_component = r#"
        {
            "id": "test2",
            "version": "1.0.0",
            "inputs": [
            ],
            "output": {
                "schema_reference": "test1@1.0.0"
            }
        }"#;

        let test1_resolver = MockComponentReferenceResolver {
            resolved: HashMap::new(),
        };

        let test2_resolver = MockComponentReferenceResolver {
            resolved: vec![(
                ComponentReference::exact("test1", "1.0.0"),
                test1_component.to_string(),
            )]
            .into_iter()
            .collect(),
        };

        let test1_resolved_components =
            resolve_components(test1_component.to_string(), Box::new(test1_resolver));

        let test2_resolved_components =
            resolve_components(test2_component.to_string(), Box::new(test2_resolver));

        println!("Test 1 failures:");
        print_failures(&test1_resolved_components);

        println!("Test 2 failures:");
        print_failures(&test2_resolved_components);

        assert_eq!(test1_resolved_components.failed.len(), 1);
        assert_eq!(test1_resolved_components.resolved.len(), 0);

        assert_eq!(test2_resolved_components.failed.len(), 1);
        assert_eq!(test2_resolved_components.resolved.len(), 1);

        let test1_component_failure = test1_resolved_components
            .failed
            .values()
            .collect::<Vec<&BuildComponentFailure>>()[0];
        match test1_component_failure {
            BuildComponentFailure::Validate {
                validation_errors,
                context,
            } => {
                assert_eq!(context[0].reference, ComponentReference::root());

                assert_eq!(validation_errors.len(), 1);
                assert_eq!(
                    validation_errors
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Error(_, _)))
                        .count(),
                    1
                );
                assert_eq!(
                    validation_errors
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Warning(_, _)))
                        .count(),
                    0
                );
            }
            _ => panic!("Unexpected failure type: {:?}", test1_component_failure),
        }

        assert_eq!(test1_resolved_components.root_warnings.len(), 1);

        let test2_component_failure = test2_resolved_components
            .failed
            .values()
            .collect::<Vec<&BuildComponentFailure>>()[0];
        match test2_component_failure {
            BuildComponentFailure::Validate {
                validation_errors,
                context,
            } => {
                assert_eq!(
                    context[0].reference,
                    ComponentReference::exact("test1", "1.0.0"),
                );

                assert_eq!(validation_errors.len(), 1);
                assert_eq!(
                    validation_errors
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Error(_, _)))
                        .count(),
                    1
                );
                assert_eq!(
                    validation_errors
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Warning(_, _)))
                        .count(),
                    0
                );
            }
            _ => panic!("Unexpected failure type: {:?}", test2_component_failure),
        }

        assert_eq!(test2_resolved_components.root_warnings.len(), 0);
    }

    #[test]
    fn it_should_return_warnings_for_root_component_on_success() {
        // This contains a duplicate name (warning), but is otherwise valid.
        let test_component = r#"
        {
            "id": "test1",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input One",
                    "default_value": 1
                },
                {
                    "id": "input2",
                    "name": "Input One",
                    "default_value": 2
                }
            ],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test_resolver = MockComponentReferenceResolver {
            resolved: HashMap::new(),
        };

        let resolved_components =
            resolve_components(test_component.to_string(), Box::new(test_resolver));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 0);
        assert_eq!(resolved_components.resolved.len(), 1);

        assert_eq!(resolved_components.root_warnings.len(), 1);
    }

    #[test]
    fn it_should_detect_circular_references_ignoring_versions() {
        // The circular reference here is going to be:
        // test@1.0.0 -> test2@1.0.0 -> test4@1.0.0 -> test2@5.0.0

        let root_component = r#"
        {
            "id": "test",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": {
                            "id": "test2",
                            "version": "1.0.0"
                        }
                    }
                }
            ],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test2_v1_component = r#"
        {
            "id": "test2",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": {
                            "id": "test3",
                            "version": "1.0.0"
                        },
                        "input_overrides": [
                            {
                                "id": "input1",
                                "component": {
                                    "reference": {
                                        "id": "test4",
                                        "version": "1.0.0"
                                    }
                                }
                            }
                        ]
                    }
                }
            ],
            "output": {
                "schema_reference": "test3@2.0.0"
            }
        }"#;

        let test3_v1_component = r#"
        {
            "id": "test3",
            "version": "1.0.0",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        let test3_v2_component = r#"
        {
            "id": "test3",
            "version": "2.0.0",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        // This contains a reference back to a different version of test2.
        let test4_v1_component = r#"
        {
            "id": "test4",
            "version": "1.0.0",
            "inputs": [],
            "output": {
                "schema_reference": "test2@5.0.0"
            }
        }"#;

        let resolver = MockComponentReferenceResolver {
            resolved: vec![
                (
                    ComponentReference::exact("test2", "1.0.0"),
                    test2_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "1.0.0"),
                    test3_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "2.0.0"),
                    test3_v2_component.to_string(),
                ),
                (
                    ComponentReference::exact("test4", "1.0.0"),
                    test4_v1_component.to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        };

        let resolved_components =
            resolve_components(root_component.to_string(), Box::new(resolver));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 1);
        assert_eq!(resolved_components.resolved.len(), 4);

        let component_failure = resolved_components
            .failed
            .values()
            .collect::<Vec<&BuildComponentFailure>>()[0];
        match component_failure {
            BuildComponentFailure::CircularReference {
                context: _,
                reference: circular_reference,
            } => {
                assert_eq!(
                    circular_reference,
                    &ComponentReference::exact("test2", "5.0.0")
                );
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
    }

    #[test]
    fn it_should_detect_circular_reference_to_root() {
        // This contains a reference to itself.
        let test_component = r#"
        {
            "id": "test1",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input One",
                    "default_value": 1
                }
            ],
            "output": {
                "schema_reference": "test1@1.0.0"
            }
        }"#;

        let test_resolver = MockComponentReferenceResolver {
            resolved: HashMap::new(),
        };

        let resolved_components =
            resolve_components(test_component.to_string(), Box::new(test_resolver));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 1);
        assert_eq!(resolved_components.resolved.len(), 0);

        let component_failure = resolved_components
            .failed
            .values()
            .collect::<Vec<&BuildComponentFailure>>()[0];
        match component_failure {
            BuildComponentFailure::CircularReference {
                context: _,
                reference: circular_reference,
            } => {
                assert_eq!(
                    circular_reference,
                    &ComponentReference::exact("test1", "1.0.0")
                );
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
    }

    struct MockComponentReferenceResolver {
        resolved: HashMap<ComponentReference, String>,
    }

    #[async_trait]
    impl ComponentReferenceResolver for MockComponentReferenceResolver {
        async fn resolve<'a, 'b>(
            &self,
            reference: ComponentReference,
            context: &'a BuildContext<'a>,
        ) -> Result<ResolvedReferenceContent<'b>, ComponentReferenceResolveError<'b>>
        where
            'a: 'b,
        {
            match self.resolved.get(&reference) {
                Some(rigging) => Ok(ResolvedReferenceContent {
                    context,
                    rigging: rigging.clone(),
                }),
                None => Err(ComponentReferenceResolveError {
                    context,
                    source: SlipwayError::RiggingResolveFailed(
                        "MockComponentReferenceResolver does not have a rigging for this reference"
                            .to_string(),
                    ),
                }),
            }
        }
    }
}
