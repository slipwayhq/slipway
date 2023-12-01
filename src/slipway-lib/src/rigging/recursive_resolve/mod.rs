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
        types::{Component, ResolvedComponentReference, UnresolvedComponentReference},
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
    resolved: HashMap<UnresolvedComponentReference, Component>,
    failed: HashMap<UnresolvedComponentReference, ResolveComponentFailure>,
    root_warnings: Vec<ValidationFailure>,
}

// This will recursively load, parse and validate the entire
// component tree.
// One current limitation is it has the potential to redundantly load
// components twice if they are reference using version ranges, because
// we match on the reference in the rigging, not the resolved reference.
// This could potentially be improved, but is an inefficiency rather
// than an error.
// The loader implementation itself may be the simplest to improve this
// as it could cache resolved components and match version ranges to previously
// loaded components.
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
        reference: UnresolvedComponentReference::Root,
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

    // Set up a map of seen references, so we don't load them twice.
    // We don't need to include the root component because it is a validation
    // error to reference ComponentReference::ROOT_ID, and a circular reference
    // error to reference the resolved root reference.
    let mut seen_references = HashMap::new();

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

        let result = resolve_component(next);

        let warnings = match result {
            Err(e) => {
                // The component failed to be completely resolved, so add it to the failed list.
                failed.insert(e.context.reference.clone(), e.failure);

                e.warnings
            }
            Ok(result) => {
                // The component was successfully resolved, so add it to the resolved list.
                resolved.insert(result.context.reference.clone(), result.component);

                // Process each reference found in the component.
                for reference in result.found_references {
                    // If we've seen this reference already, skip it.
                    if seen_references.contains_key(&reference) {
                        continue;
                    }

                    // Otherwise add it to the list of seen references.
                    seen_references.insert(reference.clone(), ());

                    // Create a new context for the reference.
                    let new_context = context_arena.alloc(Context {
                        reference: reference.clone(),
                        resolved_reference: OnceLock::new(),
                        previous_context: Some(result.context),
                    });

                    // Load the component rigging and add it to the list of futures.
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
    found_references: Vec<UnresolvedComponentReference>,
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
    let validation_result = validate_component(&unresolved_reference, &component);

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

    // Check for circular references using the resolved reference.
    if context.contains_any_resolved_version(&resolved_reference) {
        return Err(ResolvedComponentError {
            context,
            warnings,
            failure: ResolveComponentFailure::CircularResolvedReference {
                reference: resolved_reference.clone(),
                context: context.as_list(),
            },
        });
    }

    // Update the context with the resolved reference.
    // We must do this after checking for a circular resolved reference
    // or we will think the current context matches the resolved reference.
    context
        .resolved_reference
        .set(resolved_reference.clone())
        .unwrap_or_else(|resolved_reference| {
            panic!(
                r#"Resolved component reference for "{unresolved_reference}" should only be set once (setting to {resolved_reference}, existing was {})"#,
                context.resolved_reference.get().expect("Resolved reference should have been set"),
            )
        });

    // Find components referenced by this component.
    let found_references = find_component_references(&component);

    // Check for circular references using the unresolved versions.
    // This is important to do because if an unresolved reference in this component
    // matches an unresolved reference in the context then the resolved reference will have
    // been cached and won't be re-resolved, which means the circular reference check on
    // resolved references above will not occur.
    let unresolved_circular_reference = found_references
        .iter()
        .find(|reference| context.contains_any_unresolved_version(reference));

    if let Some(circular_reference) = unresolved_circular_reference {
        return Err(ResolvedComponentError {
            context,
            warnings,
            failure: ResolveComponentFailure::CircularUnresolvedReference {
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
    CircularResolvedReference {
        reference: ResolvedComponentReference,
        context: Vec<ContextSnapshot>,
    },
    CircularUnresolvedReference {
        reference: UnresolvedComponentReference,
        context: Vec<ContextSnapshot>,
    },
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Condvar, Mutex, MutexGuard,
        },
        thread::spawn,
        time::Duration,
    };

    use async_trait::async_trait;
    use semver::Version;
    use serde_json::json;
    use url::Url;

    use crate::rigging::parse::types::{
        ComponentInput, ComponentInputOverride, ComponentInputSpecification, ComponentOutput,
    };

    use super::*;

    fn print_failures(resolved_components: &ResolvedComponents) {
        resolved_components
            .failed
            .iter()
            .for_each(|e| println!("Failure: {:?}", e));
    }

    #[test]
    fn it_should_resolve_all_references() {
        let root_component = serde_json::to_string(&Component::for_test(
            "test",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("test2", "1"),
                    None,
                )),
                None,
            )],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let test2_v1_component = serde_json::to_string(&Component::for_test(
            "test2",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("test3", "1"),
                    Some(vec![ComponentInputOverride::for_test_with_component(
                        "input1",
                        ComponentInputSpecification::for_test(
                            UnresolvedComponentReference::for_test("test4", "1"),
                            None,
                        ),
                    )]),
                )),
                None,
            )],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test3", "2")),
            ),
        ))
        .unwrap();

        let test3_v1_component = serde_json::to_string(&Component::for_test(
            "test3",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let test3_v2_component = serde_json::to_string(&Component::for_test(
            "test3",
            Version::new(2, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        // This contains a reference back to a different version of test2.
        let test4_v1_component = serde_json::to_string(&Component::for_test(
            "test4",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let loader = LoadComponentRiggingMock {
            resolved: vec![
                (
                    UnresolvedComponentReference::for_test("test2", "1"),
                    test2_v1_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test3", "1"),
                    test3_v1_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test3", "2"),
                    test3_v2_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test4", "1"),
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
        let test1_component = serde_json::to_string(&Component::for_test(
            "test1",
            Version::new(1, 0, 0),
            vec![
                ComponentInput::for_test_with_display_name("input1", "Input One"),
                ComponentInput::for_test_with_display_name("input2", "Input One"),
                ComponentInput::for_test_with_display_name("input2", "Input Two"),
            ],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("foo", "1")),
            ),
        ))
        .unwrap();

        // This just references test1.
        let test2_component = serde_json::to_string(&Component::for_test(
            "test2",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test1", "1")),
            ),
        ))
        .unwrap();

        let test1_loader = LoadComponentRiggingMock {
            resolved: HashMap::new(),
        };

        let test2_loader = LoadComponentRiggingMock {
            resolved: vec![(
                UnresolvedComponentReference::for_test("test1", "1"),
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
                assert_eq!(context[0].reference, UnresolvedComponentReference::Root);

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
                    UnresolvedComponentReference::for_test("test1", "1"),
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
        let test_component = serde_json::to_string(&Component::for_test(
            "test1",
            Version::new(1, 0, 0),
            vec![
                ComponentInput::for_test_with_display_name("input1", "Input One"),
                ComponentInput::for_test_with_display_name("input2", "Input One"),
            ],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

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
    fn it_should_detect_circular_unresolved_references_ignoring_versions() {
        // The circular reference here is going to be:
        // test#1 -> test2#1 -> test4#1 -> test2#5

        let root_component = serde_json::to_string(&Component::for_test(
            "test",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("test2", "1"),
                    None,
                )),
                None,
            )],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let test2_v1_component = serde_json::to_string(&Component::for_test(
            "test2",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("test3", "1"),
                    Some(vec![ComponentInputOverride::for_test_with_component(
                        "input1",
                        ComponentInputSpecification::for_test(
                            UnresolvedComponentReference::for_test("test4", "1"),
                            None,
                        ),
                    )]),
                )),
                None,
            )],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test3", "2")),
            ),
        ))
        .unwrap();

        let test3_v1_component = serde_json::to_string(&Component::for_test(
            "test3",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let test3_v2_component = serde_json::to_string(&Component::for_test(
            "test3",
            Version::new(2, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        // This contains a reference back to a different version of test2.
        let test4_v1_component = serde_json::to_string(&Component::for_test(
            "test4",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test2", "5")),
            ),
        ))
        .unwrap();

        let loader = LoadComponentRiggingMock {
            resolved: vec![
                (
                    UnresolvedComponentReference::for_test("test2", "1"),
                    test2_v1_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test3", "1"),
                    test3_v1_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test3", "2"),
                    test3_v2_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::for_test("test4", "1"),
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
            ResolveComponentFailure::CircularUnresolvedReference {
                context: _,
                reference: circular_reference,
            } => {
                assert_eq!(
                    circular_reference,
                    &UnresolvedComponentReference::for_test("test2", "5")
                );
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
    }

    #[test]
    fn it_should_detect_circular_resolved_references_ignoring_versions() {
        // The circular reference here is going to be:
        // test#1 -> test2#1 -> test1#2

        let root_component = serde_json::to_string(&Component::for_test(
            "test",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("test2", "1"),
                    None,
                )),
                None,
            )],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        let test2_v1_component = serde_json::to_string(&Component::for_test(
            "test2",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::Url {
                        url: Url::parse("https://blah/some-component").unwrap(),
                    },
                    None,
                )),
                None,
            )],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        // This contains a reference back to a different version of test.
        let some_component = serde_json::to_string(&Component::for_test(
            "test",
            Version::new(2, 0, 0),
            vec![],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test2", "5")),
            ),
        ))
        .unwrap();

        let loader = LoadComponentRiggingMock {
            resolved: vec![
                (
                    UnresolvedComponentReference::for_test("test2", "1"),
                    test2_v1_component.to_string(),
                ),
                (
                    UnresolvedComponentReference::Url {
                        url: Url::parse("https://blah/some-component").unwrap(),
                    },
                    some_component.to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        };

        let resolved_components =
            recursively_resolve_components(root_component.to_string(), Box::new(loader));

        print_failures(&resolved_components);

        assert_eq!(resolved_components.failed.len(), 1);
        assert_eq!(resolved_components.resolved.len(), 2);

        let component_failure = resolved_components
            .failed
            .values()
            .collect::<Vec<&ResolveComponentFailure>>()[0];
        match component_failure {
            ResolveComponentFailure::CircularResolvedReference {
                context: _,
                reference: circular_reference,
            } => {
                assert_eq!(
                    circular_reference,
                    &ResolvedComponentReference::for_test("test", Version::new(2, 0, 0))
                );
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
    }

    #[test]
    fn it_should_detect_circular_reference_to_root() {
        // This contains a reference to itself.
        let test_component = serde_json::to_string(&Component::for_test(
            "test1",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test("input1", None, Some(json!(1)))],
            ComponentOutput::for_test(
                None,
                Some(UnresolvedComponentReference::for_test("test1", "1")),
            ),
        ))
        .unwrap();

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
            ResolveComponentFailure::CircularUnresolvedReference {
                context: _,
                reference: circular_reference,
            } => {
                assert_eq!(
                    circular_reference,
                    &UnresolvedComponentReference::for_test("test1", "1")
                );
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
    }

    struct LoadComponentRiggingMock {
        resolved: HashMap<UnresolvedComponentReference, String>,
    }

    #[async_trait]
    impl LoadComponentRigging for LoadComponentRiggingMock {
        async fn load_component_rigging<'a, 'b>(
            &self,
            reference: UnresolvedComponentReference,
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
                    source: SlipwayError::RiggingResolveFailed(format!(
                        "MockComponentReferenceResolver does not have a rigging for reference: {}",
                        reference
                    )),
                }),
            }
        }
    }

    // This test checks that we don't try to resolve the same reference twice if two different
    // components reference it, and that they are both processed before the referenced component has loaded.
    #[test]
    fn it_should_not_resolve_references_twice_if_they_are_in_the_process_of_being_loaded() {
        // A completes, returns references to B and C.
        let a = serde_json::to_string(&Component::for_test(
            "a",
            Version::new(1, 0, 0),
            vec![ComponentInput::for_test(
                "input1",
                Some(ComponentInputSpecification::for_test(
                    UnresolvedComponentReference::for_test("b", "1"),
                    None,
                )),
                None,
            )],
            ComponentOutput::for_test(None, Some(UnresolvedComponentReference::for_test("c", "1"))),
        ))
        .unwrap();

        let b = serde_json::to_string(&Component::for_test(
            "b",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test(None, Some(UnresolvedComponentReference::for_test("d", "1"))),
        ))
        .unwrap();

        let c = serde_json::to_string(&Component::for_test(
            "c",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test(None, Some(UnresolvedComponentReference::for_test("d", "1"))),
        ))
        .unwrap();

        // Once both B and C have loaded, D will load.
        // D should not get requested from the loader twice.
        let d = serde_json::to_string(&Component::for_test(
            "d",
            Version::new(1, 0, 0),
            vec![],
            ComponentOutput::for_test_with_schema(),
        ))
        .unwrap();

        // Set up the references, one set for this thread and one for the background thread.
        let b_ref = UnresolvedComponentReference::for_test("b", "1");
        let c_ref = UnresolvedComponentReference::for_test("c", "1");
        let d_ref = UnresolvedComponentReference::for_test("d", "1");
        let b_ref_thread = b_ref.clone();
        let c_ref_thread = c_ref.clone();
        let d_ref_thread = d_ref.clone();

        // Set up oneshot channels for B, C and D, which we will use to "load" each component.
        let (b_sender, b_receiver) = oneshot::channel();
        let (c_sender, c_receiver) = oneshot::channel();
        let (d_sender, d_receiver) = oneshot::channel();

        // Set up a list of calls to the loader. This uses a Condvar so that the test thread can
        // wait for the background thread to make the calls.
        let calls = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let calls_on_thread = calls.clone();

        let recursive_resolve_complete = Arc::new(AtomicBool::new(false));
        let recursive_resolve_complete_thread = recursive_resolve_complete.clone();

        // Set up the background thread which will run the loader, and make sure they all load successfully.
        let runner_thread = spawn(move || {
            // Create our mock loader.
            let loader = Box::new(LoadComponentRiggingDelayMock {
                resolved: Mutex::new(
                    vec![
                        (b_ref_thread, b_receiver),
                        (c_ref_thread, c_receiver),
                        (d_ref_thread, d_receiver),
                    ]
                    .into_iter()
                    .collect(),
                ),

                calls: calls_on_thread,
            });

            // Resolve all the components.
            let resolved_components = recursively_resolve_components(a.to_string(), loader);
            recursive_resolve_complete_thread.store(true, Ordering::SeqCst);

            print_failures(&resolved_components);

            // Make sure all the components were resolved successfully.
            assert_eq!(resolved_components.failed.len(), 0);
            assert_eq!(resolved_components.resolved.len(), 4);
        });

        let (lock, cvar) = &*calls;
        {
            let mut calls = lock.lock().unwrap();

            // Helper function to wait until the specified number of loader calls have been made.
            fn wait_for_calls<'a>(
                mut calls: MutexGuard<'a, Vec<UnresolvedComponentReference>>,
                // lock: &Mutex<Vec<ComponentReference>>,
                cvar: &Condvar,
                call_count: usize,
                recursive_resolve_complete: &AtomicBool,
            ) -> MutexGuard<'a, Vec<UnresolvedComponentReference>> {
                // Wait for the background thread to make the requested number of calls.
                let timeout_duration = Duration::from_millis(100);
                while (*calls).len() < call_count {
                    let (new_calls, _) = cvar.wait_timeout(calls, timeout_duration).unwrap();
                    calls = new_calls;

                    // If recursive_resolve_complete is true, the runner thread
                    // exited prematurely.
                    if recursive_resolve_complete.load(Ordering::SeqCst) {
                        panic!("Recursive resolve completed before all calls were made");
                    }
                }

                // CHeck the number of load calls is expected.
                assert_eq!(calls.len(), call_count);
                calls
            }

            // Helper function to ensure a particular reference load request has been made.
            fn assert_reference_load_requested(
                calls: &MutexGuard<'_, Vec<UnresolvedComponentReference>>,
                reference: &UnresolvedComponentReference,
            ) {
                assert!(calls.iter().any(|call| call == reference));
            }

            // Once A has been processed, B and C will have load requests.
            calls = wait_for_calls(calls, cvar, 2, &recursive_resolve_complete);
            assert_reference_load_requested(&calls, &b_ref);
            assert_reference_load_requested(&calls, &c_ref);

            // Load B, which will return a reference to D.
            b_sender.send(b.to_string()).unwrap();

            // Wait D to have a load request.
            calls = wait_for_calls(calls, cvar, 3, &recursive_resolve_complete);
            assert_reference_load_requested(&calls, &d_ref);

            // Next we load C, which will also return a reference to D, before
            // the previous D load request has completed.
            c_sender.send(c.to_string()).unwrap();

            // If this results in two attempts to load D, we will get a LoadError because
            // we're removing each receiver from the Loader's map when we use it.

            // Finally, we can load D to allow the background thread to complete.
            d_sender.send(d.to_string()).unwrap();
        }

        // Wait for the background thread to complete.
        runner_thread.join().unwrap();

        // Check that the loader was only called once for each reference.
        let total_calls = lock.lock().unwrap().len();
        assert_eq!(total_calls, 3);
    }

    // Mock loader which waits for a oneshot channel to be sent to before returning
    // the rigging for a reference.
    struct LoadComponentRiggingDelayMock {
        resolved: Mutex<HashMap<UnresolvedComponentReference, oneshot::Receiver<String>>>,
        calls: Arc<(Mutex<Vec<UnresolvedComponentReference>>, Condvar)>,
    }

    #[async_trait]
    impl LoadComponentRigging for LoadComponentRiggingDelayMock {
        async fn load_component_rigging<'a, 'b>(
            &self,
            reference: UnresolvedComponentReference,
            context: &'a Context<'a>,
        ) -> Result<ComponentRigging<'b>, LoadError<'b>>
        where
            'a: 'b,
        {
            // Add the reference to the list of calls, and notify the test thread that
            // a call has been made. We do this before awaiting the receiver so that the
            // test thread can know the call has been made before the component is "loaded".
            let (lock, cvar) = &*self.calls;
            {
                let mut started = lock.lock().unwrap();
                started.push(reference.clone());
                cvar.notify_one();
            }

            // Remove the receiver from the map, which will cause the next call to this method
            // with the same reference to fail with a LoadError below.
            // We must remove it because we need to release the lock on the hashmap before awaiting
            // the receiver.
            let receiver = self.resolved.lock().unwrap().remove(&reference);

            match receiver {
                Some(receiver) => {
                    // Wait for the receiver to be sent the rigging.
                    let rigging = receiver.await.unwrap();
                    Ok(ComponentRigging { context, rigging })
                }
                None => Err(LoadError {
                    context,
                    source: SlipwayError::RiggingResolveFailed(format!(
                        r#"loader does not have rigging for reference {:?}. Either it never existed, or it was loaded twice."#,
                        reference
                    )),
                }),
            }
        }
    }
}
