use std::sync::OnceLock;

use crate::rigging::parse::types::{ResolvedComponentReference, UnresolvedComponentReference};

#[derive(Debug, Clone)]
pub(crate) struct Context<'a> {
    pub reference: UnresolvedComponentReference,

    // We're using OnceLock rather than OnceCell so that the context is Send
    // and we can use it in a future.
    pub resolved_reference: OnceLock<ResolvedComponentReference>,

    pub previous_context: Option<&'a Context<'a>>,
}

impl<'a> Context<'a> {
    pub fn as_list(&self) -> Vec<ContextSnapshot> {
        let mut result = vec![self.to_snapshot()];

        let mut current_context = self.previous_context;
        while let Some(context) = current_context {
            result.push(context.to_snapshot());
            current_context = context.previous_context;
        }

        result
    }

    pub fn contains_any_resolved_version(&self, reference: &ResolvedComponentReference) -> bool {
        let context_resolved_reference = self.resolved_reference.get();
        if let Some(context_resolved_reference) = context_resolved_reference {
            if context_resolved_reference.publisher == reference.publisher
                && context_resolved_reference.name == reference.name
            {
                return true;
            }
        }

        if let Some(previous_context) = &self.previous_context {
            return previous_context.contains_any_resolved_version(reference);
        }

        false
    }

    pub fn contains_any_unresolved_version(
        &self,
        reference: &UnresolvedComponentReference,
    ) -> bool {
        let found = match reference {
            UnresolvedComponentReference::Root => {
                matches!(&self.reference, UnresolvedComponentReference::Root)
            }
            UnresolvedComponentReference::Registry {
                publisher, name, ..
            } => {
                (match &self.reference {
                    UnresolvedComponentReference::Registry {
                        publisher: other_publisher,
                        name: other_name,
                        ..
                    } => publisher == other_publisher && name == other_name,
                    UnresolvedComponentReference::GitHub {
                        user, repository, ..
                    } => publisher == user && name == repository,
                    _ => false,
                }) || (match self.resolved_reference.get() {
                    Some(resolved_reference) => {
                        publisher == &resolved_reference.publisher
                            && name == &resolved_reference.name
                    }
                    None => false,
                })
            }
            UnresolvedComponentReference::GitHub {
                user, repository, ..
            } => {
                (match &self.reference {
                    UnresolvedComponentReference::GitHub {
                        user: other_user,
                        repository: other_repository,
                        ..
                    } => user == other_user && repository == other_repository,
                    UnresolvedComponentReference::Registry {
                        publisher, name, ..
                    } => publisher == user && name == repository,
                    _ => false,
                }) || (match self.resolved_reference.get() {
                    Some(resolved_reference) => {
                        user == &resolved_reference.publisher
                            && repository == &resolved_reference.name
                    }
                    None => false,
                })
            }
            UnresolvedComponentReference::Local { path, .. } => match &self.reference {
                UnresolvedComponentReference::Local {
                    path: other_path, ..
                } => path == other_path,
                _ => false,
            },
            UnresolvedComponentReference::Url { url, .. } => match &self.reference {
                UnresolvedComponentReference::Url { url: other_url, .. } => url == other_url,
                _ => false,
            },
        };

        if found {
            return true;
        }

        if let Some(previous_context) = &self.previous_context {
            return previous_context.contains_any_unresolved_version(reference);
        }

        false
    }

    pub fn to_snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            reference: self.reference.clone(),
            resolved_reference: self.resolved_reference.get().cloned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextSnapshot {
    pub reference: UnresolvedComponentReference,
    pub resolved_reference: Option<ResolvedComponentReference>,
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;

    #[test]
    fn get_list_should_return_list_of_references_in_current_context() {
        let context_0 = Context {
            reference: UnresolvedComponentReference::for_test("context-0", "1"),
            resolved_reference: OnceLock::from(ResolvedComponentReference::for_test(
                "context-0-resolved",
                Version::new(1, 0, 0),
            )),
            previous_context: None,
        };
        let context_1 = Context {
            reference: UnresolvedComponentReference::for_test("context-1", "1"),
            resolved_reference: OnceLock::new(),
            previous_context: Some(&context_0),
        };

        let list = context_1.as_list();

        assert_eq!(list, vec![context_1.to_snapshot(), context_0.to_snapshot()]);
    }

    #[test]
    fn contains_reference_it_should_return_true_if_context_contains_specified_reference() {
        let context_0 = Context {
            reference: UnresolvedComponentReference::Root,
            resolved_reference: OnceLock::from(ResolvedComponentReference::for_test(
                "context-0-resolved",
                Version::new(1, 0, 0),
            )),
            previous_context: None,
        };

        let context_1 = Context {
            reference: UnresolvedComponentReference::for_test("context-1", "1"),
            resolved_reference: OnceLock::new(),
            previous_context: Some(&context_0),
        };

        let context_2 = Context {
            reference: UnresolvedComponentReference::for_test("context-2", "1"),
            resolved_reference: OnceLock::from(ResolvedComponentReference::for_test(
                "context-2-resolved",
                Version::new(1, 0, 0),
            )),
            previous_context: Some(&context_1),
        };

        assert!(
            context_2.contains_any_resolved_version(&ResolvedComponentReference::for_test(
                "context-0-resolved",
                Version::new(1, 0, 0)
            ))
        );
        assert!(
            context_2.contains_any_resolved_version(&ResolvedComponentReference::for_test(
                "context-2-resolved",
                Version::new(2, 0, 0)
            ))
        );

        assert!(
            !context_2.contains_any_resolved_version(&ResolvedComponentReference::new(
                "foo",
                "context-0-resolved",
                &Version::new(1, 0, 0)
            ))
        );
        assert!(
            !context_2.contains_any_resolved_version(&ResolvedComponentReference::for_test(
                "context-1",
                Version::new(2, 0, 0)
            ))
        );

        assert!(context_2.contains_any_unresolved_version(&UnresolvedComponentReference::Root));
        assert!(context_2.contains_any_unresolved_version(
            &UnresolvedComponentReference::for_test("context-1", "1")
        ));
        assert!(context_2.contains_any_unresolved_version(
            &UnresolvedComponentReference::for_test("context-1", "2")
        ));
    }
}
