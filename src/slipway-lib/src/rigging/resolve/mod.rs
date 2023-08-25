use std::{collections::HashMap, fmt::Display, sync::Arc};

use async_executor::LocalExecutor;
use async_trait::async_trait;
use futures_lite::{future, FutureExt};
use thiserror::Error;

use crate::errors::SlipwayError;
use crate::rigging::find_component_references::find_component_references;

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
}

async fn resolve_components_async(
    root_component: String,
    component_reference_resolver: Box<dyn ComponentReferenceResolver>,
) -> ResolvedComponents {
    // This is a list of component references which are in the process of being
    // fetched, or possibly have completed being fetched and are waiting to be
    // processed. We add the root component to the list as the first item to process.
    let mut futures = vec![future::ready(Result::<
        ResolvedReference,
        ComponentReferenceResolveError,
    >::Ok(ResolvedReference {
        context: Arc::new(BuildContext {
            reference: ComponentReference::root(),
            previous_context: None,
        }),
        rigging: root_component,
    }))
    .boxed()];

    let mut validated = HashMap::new();
    let mut failed = HashMap::new();

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
                        path: e.context.get_list(),
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
                                path: context.get_list(),
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

                        let mut failures = validation_result.failures;

                        if !is_root {
                            failures.retain(|f| matches!(f, ValidationFailure::Error(_, _)));
                        }

                        if !failures.is_empty() {
                            failed.insert(
                                context_component_reference.clone(),
                                BuildComponentFailure::Validate {
                                    validation_failures: failures,
                                    path: context.get_list(),
                                },
                            );
                        } else {
                            let references = find_component_references(&component);

                            validated.insert(context_component_reference.clone(), component);

                            for reference in references {
                                if validated.contains_key(&reference)
                                    || failed.contains_key(&reference)
                                {
                                    break;
                                }

                                let context = BuildContext {
                                    reference: reference.clone(),
                                    previous_context: Some(Arc::clone(&context)),
                                };

                                futures
                                    .push(component_reference_resolver.resolve(reference, context));
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
    }
}

#[derive(Debug)]
enum BuildComponentFailure {
    Resolve {
        source: SlipwayError,
        path: Vec<ComponentReference>,
    },
    Parse {
        source: SlipwayError,
        path: Vec<ComponentReference>,
    },
    Validate {
        validation_failures: Vec<ValidationFailure>,
        path: Vec<ComponentReference>,
    },
}

#[derive(Error, Debug)]
#[error("Rigging parse failed")]
pub(crate) struct ComponentReferenceResolveError {
    pub context: Arc<BuildContext>,
    pub source: SlipwayError,
}

#[async_trait]
pub(crate) trait ComponentReferenceResolver {
    async fn resolve(
        &self,
        reference: ComponentReference,
        context: BuildContext,
    ) -> Result<ResolvedReference, ComponentReferenceResolveError>;
}

pub(crate) struct ResolvedReference {
    pub context: Arc<BuildContext>,
    pub rigging: String,
}

#[derive(Debug, Clone)]
pub(crate) struct BuildContext {
    pub reference: ComponentReference,
    pub previous_context: Option<Arc<BuildContext>>,
}

impl BuildContext {
    pub fn get_path(&self) -> String {
        let mut path = self.reference.to_string();

        let mut current_context = self.previous_context.clone();
        while let Some(context) = current_context {
            path.insert_str(0, " > ");
            path.insert_str(0, &context.reference.to_string());
            current_context = context.previous_context.clone();
        }

        path
    }

    pub fn get_list(&self) -> Vec<ComponentReference> {
        let mut result = vec![self.reference.clone()];

        let mut current_context = self.previous_context.clone();
        while let Some(context) = current_context {
            result.push(context.reference.clone());
            current_context = context.previous_context.clone();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hash;

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
                "schema_reference": "test3:2.0.0"
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
                    ComponentReference {
                        id: "test2".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    test2_v1_component.to_string(),
                ),
                (
                    ComponentReference {
                        id: "test3".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    test3_v1_component.to_string(),
                ),
                (
                    ComponentReference {
                        id: "test3".to_string(),
                        version: "2.0.0".to_string(),
                    },
                    test3_v2_component.to_string(),
                ),
                (
                    ComponentReference {
                        id: "test4".to_string(),
                        version: "1.0.0".to_string(),
                    },
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
                "schema_reference": "foo:1.0"
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
                "schema_reference": "test1:1.0.0"
            }
        }"#;

        let test1_resolver = MockComponentReferenceResolver {
            resolved: HashMap::new(),
        };

        let test2_resolver = MockComponentReferenceResolver {
            resolved: vec![(
                ComponentReference {
                    id: "test1".to_string(),
                    version: "1.0.0".to_string(),
                },
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
                validation_failures,
                path,
            } => {
                assert_eq!(path[0], ComponentReference::root());

                assert_eq!(validation_failures.len(), 2);
                assert_eq!(
                    validation_failures
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Error(_, _)))
                        .count(),
                    1
                );
                assert_eq!(
                    validation_failures
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Warning(_, _)))
                        .count(),
                    1
                );
            }
            _ => panic!("Unexpected failure type: {:?}", test1_component_failure),
        }

        let test2_component_failure = test2_resolved_components
            .failed
            .values()
            .collect::<Vec<&BuildComponentFailure>>()[0];
        match test2_component_failure {
            BuildComponentFailure::Validate {
                validation_failures,
                path,
            } => {
                assert_eq!(
                    path[0],
                    ComponentReference {
                        id: "test1".to_string(),
                        version: "1.0.0".to_string()
                    }
                );

                assert_eq!(validation_failures.len(), 1);
                assert_eq!(
                    validation_failures
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Error(_, _)))
                        .count(),
                    1
                );
                assert_eq!(
                    validation_failures
                        .iter()
                        .filter(|f| matches!(f, ValidationFailure::Warning(_, _)))
                        .count(),
                    0
                );
            }
            _ => panic!("Unexpected failure type: {:?}", test2_component_failure),
        }
    }

    #[test]
    fn it_should_return_warnings_for_root_component_on_success() {
        todo!();
    }

    #[test]
    fn it_should_resolve_root_once_for_circular_reference() {
        todo!();
    }

    struct MockComponentReferenceResolver {
        resolved: HashMap<ComponentReference, String>,
    }

    #[async_trait]
    impl ComponentReferenceResolver for MockComponentReferenceResolver {
        async fn resolve(
            &self,
            reference: ComponentReference,
            context: BuildContext,
        ) -> Result<ResolvedReference, ComponentReferenceResolveError> {
            match self.resolved.get(&reference) {
                Some(rigging) => Ok(ResolvedReference {
                    context: Arc::new(context),
                    rigging: rigging.clone(),
                }),
                None => Err(ComponentReferenceResolveError {
                    context: Arc::new(context),
                    source: SlipwayError::RiggingResolveFailed(
                        "MockComponentReferenceResolver does not have a rigging for this reference"
                            .to_string(),
                    ),
                }),
            }
        }
    }
}
