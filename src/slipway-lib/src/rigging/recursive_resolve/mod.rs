pub(crate) mod context;
pub(crate) mod load;

use std::{collections::HashMap, sync::OnceLock};

use async_executor::LocalExecutor;
use futures_lite::{future, FutureExt};
use thiserror::Error;
use typed_arena::Arena;

use crate::errors::SlipwayError;
use crate::rigging::find_component_references::find_component_references;

pub(crate) use context::Context;

use self::{
    context::ContextSnapshot,
    load::{ComponentRigging, LoadComponentRigging, LoadError},
};

use super::{
    parse::{
        parse_component,
        types::{Component, ComponentReference},
    },
    validate::{validate_component, validation_failure::ValidationFailure},
};

pub(crate) fn recursively_resolve_components(
    root_component_rigging: String,
    loader: Box<dyn LoadComponentRigging>,
) -> ResolvedComponents {
    let local_executor = LocalExecutor::new();

    future::block_on(local_executor.run(recursively_resolve_components_async(
        root_component_rigging,
        loader,
    )))
}

pub(crate) struct ResolvedComponents {
    resolved: HashMap<ComponentReference, Component>,
    failed: HashMap<ComponentReference, ResolveComponentFailure>,
    root_warnings: Vec<ValidationFailure>,
}

async fn recursively_resolve_components_async(
    root_component_rigging: String,
    loader: Box<dyn LoadComponentRigging>,
) -> ResolvedComponents {
    // We're going to create all the contexts in an arena, so they can hold
    // references to each other while all having the same lifetime.
    let context_arena = Arena::new();

    // We already have the rigging for the root component, so we can create
    // the context for it now.
    // We use ComponentReference::root() because we do not know the actual id
    // or version yet.
    let root_context = context_arena.alloc(Context {
        reference: ComponentReference::root(),
        resolved_reference: OnceLock::new(),
        previous_context: None,
    });

    // This is a list of component references which are in the process of being
    // loaded, or have completed being loaded but have not been processed.
    // We add the root component to the list as the first item to process.
    let mut loader_futures = vec![future::ready(Result::<ComponentRigging, LoadError>::Ok(
        ComponentRigging {
            context: root_context,
            rigging: root_component_rigging,
        },
    ))
    .boxed()];

    // Set up the collections to store the results.
    let mut resolved = HashMap::new();
    let mut failed = HashMap::new();
    let mut root_warnings = Vec::new();

    // The first component we resolve is always the root, as it is the only one
    // in the list of futures.
    let mut is_root_component = true;

    while !loader_futures.is_empty() {
        // When any task which has not been processed is ready, process it.
        let (next, _, remaining_futures) = futures_util::future::select_all(loader_futures).await;
        loader_futures = remaining_futures;

        let result = resolve_component(next, is_root_component);

        let warnings = match result {
            Err(e) => {
                // The component failed to be completely resolved, so add it to the failed list.
                failed.insert(e.context.reference.clone(), e.failure);

                e.warnings
            }
            Ok(result) => {
                // The component was successfully resolved, so add it to the resolved list.
                resolved.insert(result.context.reference.clone(), result.component);

                // Filter the list of found references to remove ones we've already seen.
                // TODO: Does this have a race condition if two components reference the same component?
                // Yes, if a reference is loading, it will not be in the resolved or failed lists.
                // We should keep a list of seen references and check that instead.
                // But first, create a test that fails.
                let new_references = result.found_references.into_iter().filter(|reference| {
                    !resolved.contains_key(reference) && !failed.contains_key(reference)
                });

                // For each new reference, load the component rigging and add it to the list of futures.
                for reference in new_references {
                    let new_context = context_arena.alloc(Context {
                        reference: reference.clone(),
                        resolved_reference: OnceLock::new(),
                        previous_context: Some(result.context),
                    });

                    loader_futures.push(loader.load_component_rigging(reference, new_context));
                }

                result.warnings
            }
        };

        // If this is the root component, we need to store the warnings.
        // We don't care about the warnings of other components because they are not in our control.
        if is_root_component {
            root_warnings = warnings;
        }

        // Any component after the first one is not the root component.
        is_root_component = false;
    }

    ResolvedComponents {
        resolved,
        failed,
        root_warnings,
    }
}

struct ResolvedComponentData<'a> {
    context: &'a Context<'a>,
    component: Component,
    warnings: Vec<ValidationFailure>,
    found_references: Vec<ComponentReference>,
}

#[derive(Error, Debug)]
#[error("Failed to resolve component")]
struct ResolvedComponentError<'a> {
    context: &'a Context<'a>,
    warnings: Vec<ValidationFailure>,
    failure: ResolveComponentFailure,
}

fn resolve_component<'a>(
    load_result: Result<ComponentRigging<'a>, LoadError<'a>>,
    is_root_component: bool,
) -> Result<ResolvedComponentData<'a>, ResolvedComponentError<'a>> {
    // Load.
    let result = load_result.map_err(|e| ResolvedComponentError {
        context: e.context,
        warnings: Vec::new(),
        failure: ResolveComponentFailure::Load {
            source: e.source,
            context: e.context.as_list(),
        },
    })?;

    let context = result.context;
    let unresolved_reference = context.reference.clone();

    // Parse.
    let component = parse_component(&result.rigging).map_err(|e| ResolvedComponentError {
        context,
        warnings: Vec::new(),
        failure: ResolveComponentFailure::Parse {
            source: e,
            context: context.as_list(),
        },
    })?;

    // Validate.
    let expected_component_id = match is_root_component {
        true => None,
        false => Some(unresolved_reference.id.clone()),
    };
    let validation_result = validate_component(expected_component_id, &component);

    let (warnings, errors): (Vec<ValidationFailure>, Vec<ValidationFailure>) = validation_result
        .failures
        .into_iter()
        .partition(|f| matches!(f, ValidationFailure::Warning(_, _)));

    if !errors.is_empty() {
        return Err(ResolvedComponentError {
            context,
            warnings,
            failure: ResolveComponentFailure::Validate {
                validation_errors: errors,
                context: context.as_list(),
            },
        });
    }

    // The component is valid, so we can safely read the resolved reference for it.
    let resolved_reference = component.get_reference();

    // Update the context with the resolved reference.
    // We must do this before checking for circular references.
    context
        .resolved_reference
        .set(resolved_reference)
        .unwrap_or_else(|resolved_reference| {
            panic!(
                r#"Resolved component reference for "{unresolved_reference}" should only be set once (setting to {resolved_reference}, existing was {})"#,
                context.resolved_reference.get().expect("Resolved reference should have been set"),
            )
        });

    // Find components referenced by this component.
    let found_references = find_component_references(&component);

    // Check for circular references.
    let circular_reference = found_references
        .iter()
        .find(|reference| context.contains_resolved_id(&reference.id));

    if let Some(circular_reference) = circular_reference {
        return Err(ResolvedComponentError {
            context,
            warnings,
            failure: ResolveComponentFailure::CircularReference {
                reference: circular_reference.clone(),
                context: context.as_list(),
            },
        });
    }

    // Everything looks good, so return what we found.
    Ok(ResolvedComponentData {
        context,
        component,
        warnings,
        found_references,
    })
}

#[derive(Debug)]
enum ResolveComponentFailure {
    Load {
        source: SlipwayError,
        context: Vec<ContextSnapshot>,
    },
    Parse {
        source: SlipwayError,
        context: Vec<ContextSnapshot>,
    },
    Validate {
        validation_errors: Vec<ValidationFailure>,
        context: Vec<ContextSnapshot>,
    },
    CircularReference {
        reference: ComponentReference,
        context: Vec<ContextSnapshot>,
    },
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;

    fn print_failures(resolved_components: &ResolvedComponents) {
        resolved_components
            .failed
            .iter()
            .for_each(|e| println!("{:?}", e));
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

        let loader = LoadComponentRiggingMock {
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
            recursively_resolve_components(root_component.to_string(), Box::new(loader));

        print_failures(&resolved_components);

        assert!(resolved_components.failed.is_empty());
        assert_eq!(resolved_components.resolved.len(), 5);
    }

    #[test]
    fn it_should_return_errors_and_warnings_for_root_component_on_failure_but_only_errors_for_referenced_components(
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

        let test1_loader = LoadComponentRiggingMock {
            resolved: HashMap::new(),
        };

        let test2_loader = LoadComponentRiggingMock {
            resolved: vec![(
                ComponentReference::exact("test1", "1.0.0"),
                test1_component.to_string(),
            )]
            .into_iter()
            .collect(),
        };

        let test1_resolved_components =
            recursively_resolve_components(test1_component.to_string(), Box::new(test1_loader));

        let test2_resolved_components =
            recursively_resolve_components(test2_component.to_string(), Box::new(test2_loader));

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
            .collect::<Vec<&ResolveComponentFailure>>()[0];
        match test1_component_failure {
            ResolveComponentFailure::Validate {
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
            .collect::<Vec<&ResolveComponentFailure>>()[0];
        match test2_component_failure {
            ResolveComponentFailure::Validate {
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

        let loader = LoadComponentRiggingMock {
            resolved: HashMap::new(),
        };

        let resolved_components =
            recursively_resolve_components(test_component.to_string(), Box::new(loader));

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

        let loader = LoadComponentRiggingMock {
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
            recursively_resolve_components(root_component.to_string(), Box::new(loader));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 1);
        assert_eq!(resolved_components.resolved.len(), 4);

        let component_failure = resolved_components
            .failed
            .values()
            .collect::<Vec<&ResolveComponentFailure>>()[0];
        match component_failure {
            ResolveComponentFailure::CircularReference {
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

        let loader = LoadComponentRiggingMock {
            resolved: HashMap::new(),
        };

        let resolved_components =
            recursively_resolve_components(test_component.to_string(), Box::new(loader));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 1);
        assert_eq!(resolved_components.resolved.len(), 0);

        let component_failure = resolved_components
            .failed
            .values()
            .collect::<Vec<&ResolveComponentFailure>>()[0];
        match component_failure {
            ResolveComponentFailure::CircularReference {
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

    #[test]
    fn it_should_not_resolve_references_twice_if_they_are_in_the_process_of_being_loaded() {
        // A completes, returns B and C.
        // B and C complete, both return D.
        // D should not get loaded twice.
        // We can check this by looking at how many times the loader is called.
        todo!();
    }

    struct LoadComponentRiggingMock {
        resolved: HashMap<ComponentReference, String>,
    }

    #[async_trait]
    impl LoadComponentRigging for LoadComponentRiggingMock {
        async fn load_component_rigging<'a, 'b>(
            &self,
            reference: ComponentReference,
            context: &'a Context<'a>,
        ) -> Result<ComponentRigging<'b>, LoadError<'b>>
        where
            'a: 'b,
        {
            match self.resolved.get(&reference) {
                Some(rigging) => Ok(ComponentRigging {
                    context,
                    rigging: rigging.clone(),
                }),
                None => Err(LoadError {
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
