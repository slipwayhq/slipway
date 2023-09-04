use std::sync::OnceLock;

use crate::rigging::parse::types::ComponentReference;

#[derive(Debug, Clone)]
pub(crate) struct Context<'a> {
    pub reference: ComponentReference,

    // We're using OnceLock rather than OnceCell so that the BuildContext is Send
    // so we can use it in a future.
    pub resolved_reference: OnceLock<ComponentReference>,

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

    pub fn contains_resolved_id(&self, id: &str) -> bool {
        let current_resolved_reference = self.resolved_reference.get();
        if let Some(resolved_reference) = current_resolved_reference {
            if resolved_reference.id == id {
                return true;
            }
        }

        if let Some(previous_context) = &self.previous_context {
            return previous_context.contains_resolved_id(id);
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
    pub reference: ComponentReference,
    pub resolved_reference: Option<ComponentReference>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_list_should_return_list_of_references_in_current_context() {
        let context_0 = Context {
            reference: ComponentReference::exact("context-0", "1"),
            resolved_reference: OnceLock::from(ComponentReference::exact(
                "context-0-resolved",
                "1",
            )),
            previous_context: None,
        };
        let context_1 = Context {
            reference: ComponentReference::exact("context-1", "1"),
            resolved_reference: OnceLock::new(),
            previous_context: Some(&context_0),
        };

        let list = context_1.as_list();

        assert_eq!(list, vec![context_1.to_snapshot(), context_0.to_snapshot()]);
    }

    #[test]
    fn contains_resolved_it_should_return_true_if_context_contains_specified_id() {
        let context_0 = Context {
            reference: ComponentReference::exact(ComponentReference::ROOT_ID, "1"),
            resolved_reference: OnceLock::from(ComponentReference::exact(
                "context-0-resolved",
                "1",
            )),
            previous_context: None,
        };

        let context_1 = Context {
            reference: ComponentReference::exact("context-1", "1"),
            resolved_reference: OnceLock::new(),
            previous_context: Some(&context_0),
        };

        let context_2 = Context {
            reference: ComponentReference::exact("context-2", "1"),
            resolved_reference: OnceLock::from(ComponentReference::exact(
                "context-2-resolved",
                "1",
            )),
            previous_context: Some(&context_1),
        };

        assert!(context_2.contains_resolved_id("context-0-resolved"));
        assert!(context_2.contains_resolved_id("context-2-resolved"));

        assert!(!context_2.contains_resolved_id("context-1"));
        assert!(!context_2.contains_resolved_id(ComponentReference::ROOT_ID));
    }
}
