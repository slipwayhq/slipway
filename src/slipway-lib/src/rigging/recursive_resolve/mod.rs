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
    use std::{
        sync::{Arc, Condvar, Mutex, MutexGuard},
        thread::spawn,
    };

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
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": "test2@1"
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
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": "test3@1",
                        "input_overrides": [
                            {
                                "id": "input1",
                                "component": {
                                    "reference": "test4@1"
                                }
                            }
                        ]
                    }
                }
            ],
            "output": {
                "schema_reference": "test3@2"
            }
        }"#;

        let test3_v1_component = r#"
        {
            "id": "test3",
            "version": "1",
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
            "version": "2",
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
            "version": "1",
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
                    ComponentReference::exact("test2", "1"),
                    test2_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "1"),
                    test3_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "2"),
                    test3_v2_component.to_string(),
                ),
                (
                    ComponentReference::exact("test4", "1"),
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
            "version": "1",
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
            "version": "1",
            "inputs": [
            ],
            "output": {
                "schema_reference": "test1@1"
            }
        }"#;

        let test1_loader = LoadComponentRiggingMock {
            resolved: HashMap::new(),
        };

        let test2_loader = LoadComponentRiggingMock {
            resolved: vec![(
                ComponentReference::exact("test1", "1"),
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
                    ComponentReference::exact("test1", "1"),
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
            "version": "1",
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
        // test@1 -> test2@1 -> test4@1 -> test2@5

        let root_component = r#"
        {
            "id": "test",
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": "test2@1"
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
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": "test3@1",
                        "input_overrides": [
                            {
                                "id": "input1",
                                "component": {
                                    "reference": "test4@1"
                                }
                            }
                        ]
                    }
                }
            ],
            "output": {
                "schema_reference": "test3@2"
            }
        }"#;

        let test3_v1_component = r#"
        {
            "id": "test3",
            "version": "1",
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
            "version": "2",
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
            "version": "1",
            "inputs": [],
            "output": {
                "schema_reference": "test2@5"
            }
        }"#;

        let loader = LoadComponentRiggingMock {
            resolved: vec![
                (
                    ComponentReference::exact("test2", "1"),
                    test2_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "1"),
                    test3_v1_component.to_string(),
                ),
                (
                    ComponentReference::exact("test3", "2"),
                    test3_v2_component.to_string(),
                ),
                (
                    ComponentReference::exact("test4", "1"),
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
                assert_eq!(circular_reference, &ComponentReference::exact("test2", "5"));
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
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input One",
                    "default_value": 1
                }
            ],
            "output": {
                "schema_reference": "test1@1"
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
                assert_eq!(circular_reference, &ComponentReference::exact("test1", "1"));
            }
            _ => panic!("Unexpected failure type: {:?}", component_failure),
        }
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

    // This test is going to check that we don't try to resolve the same reference twice if two different
    // components reference it, and they are both processed before the referenced component has loaded.
    #[test]
    fn it_should_not_resolve_references_twice_if_they_are_in_the_process_of_being_loaded() {
        // A completes, returns references to B and C.
        let a = r#"
        {
            "id": "a",
            "version": "1",
            "inputs": [
                {
                    "id": "input1",
                    "default_component": {
                        "reference": "b@1"
                    }
                }
            ],
            "output": {
                "schema_reference": "c@1"
            }
        }"#;

        // B and C will load next, both return a reference to D.
        let b = r#"
        {
            "id": "b",
            "version": "1",
            "inputs": [],
            "output": {
                "schema_reference": "d@1"
            }
        }"#;
        let c = r#"
        {
            "id": "c",
            "version": "1",
            "inputs": [],
            "output": {
                "schema_reference": "d@1"
            }
        }"#;

        // Once both B adn C have loaded, D will load.
        // D should not get requested from the loader twice.
        let d = r#"
        {
            "id": "d",
            "version": "1",
            "inputs": [],
            "output": {
                "schema": {
                    "type": "string"
                }
            }
        }"#;

        // Set up the references, one set for this thread and one for the background thread.
        let b_ref = ComponentReference::exact("b", "1");
        let c_ref = ComponentReference::exact("c", "1");
        let d_ref = ComponentReference::exact("d", "1");
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
                mut calls: MutexGuard<'a, Vec<ComponentReference>>,
                // lock: &Mutex<Vec<ComponentReference>>,
                cvar: &Condvar,
                call_count: usize,
            ) -> MutexGuard<'a, Vec<ComponentReference>> {
                // Wait for the background thread to make the requested number of calls.
                while (*calls).len() < call_count {
                    calls = cvar.wait(calls).unwrap();
                }

                // CHeck the number of load calls is expected.
                assert_eq!(calls.len(), call_count);
                calls
            }

            // Helper function to ensure a particular reference load request has been made.
            fn assert_reference_load_requested(
                calls: &MutexGuard<'_, Vec<ComponentReference>>,
                reference: &ComponentReference,
            ) {
                assert!(calls.iter().any(|call| call == reference));
            }

            // Once A has been processed, B and C will have load requests.
            calls = wait_for_calls(calls, cvar, 2);
            assert_reference_load_requested(&calls, &b_ref);
            assert_reference_load_requested(&calls, &c_ref);

            // Load B, which will return a reference to D.
            b_sender.send(b.to_string()).unwrap();

            // Wait D to have a load request.
            calls = wait_for_calls(calls, cvar, 3);
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
        resolved: Mutex<HashMap<ComponentReference, oneshot::Receiver<String>>>,
        calls: Arc<(Mutex<Vec<ComponentReference>>, Condvar)>,
    }

    #[async_trait]
    impl LoadComponentRigging for LoadComponentRiggingDelayMock {
        async fn load_component_rigging<'a, 'b>(
            &self,
            reference: ComponentReference,
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
                        r#"Loader does not have rigging for reference {:?}. Either it never existed, or it was loaded twice."#,
                        reference
                    )),
                }),
            }
        }
    }
}
