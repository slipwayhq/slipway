use std::sync::{Arc, OnceLock};

use crate::rigging::parse::types::ComponentReference;

#[derive(Debug, Clone)]
pub(crate) struct BuildContext {
    pub reference: ComponentReference,
    pub resolved_reference: OnceLock<ComponentReference>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_path_should_return_path_to_current_context_when_one_context_node() {
        let context = BuildContext {
            reference: ComponentReference {
                id: "root".to_string(),
                version: "1.0.0".to_string(),
            },
            resolved_reference: OnceLock::new(),
            previous_context: None,
        };

        assert_eq!("root:1.0.0", context.get_path());
    }

    #[test]
    fn get_path_should_return_path_to_current_context_when_multiple_context_nodes() {
        let context = BuildContext {
            reference: ComponentReference {
                id: "root".to_string(),
                version: "1.0.0".to_string(),
            },
            resolved_reference: OnceLock::new(),
            previous_context: Some(Arc::new(BuildContext {
                reference: ComponentReference {
                    id: "child".to_string(),
                    version: "1.0.0".to_string(),
                },
                resolved_reference: OnceLock::new(),
                previous_context: None,
            })),
        };

        assert_eq!("child:1.0.0 > root:1.0.0", context.get_path());
    }

    #[test]
    fn get_list_should_return_list_of_references_in_current_context() {
        let context = BuildContext {
            reference: ComponentReference {
                id: "root".to_string(),
                version: "1.0.0".to_string(),
            },
            resolved_reference: OnceLock::new(),
            previous_context: Some(Arc::new(BuildContext {
                reference: ComponentReference {
                    id: "child".to_string(),
                    version: "1.0.0".to_string(),
                },
                resolved_reference: OnceLock::new(),
                previous_context: None,
            })),
        };

        let list = context.get_list();

        assert_eq!(2, list.len());
        assert_eq!("root", list[0].id);
        assert_eq!("child", list[1].id);
    }

    #[test]
    fn contains_resolved_it_should_return_true_if_context_contains_specified_id() {
        let context = BuildContext {
            reference: ComponentReference {
                id: ComponentReference::ROOT_ID.to_string(),
                version: "1.0.0".to_string(),
            },
            resolved_reference: OnceLock::from(ComponentReference {
                id: "my-component".to_string(),
                version: "1.0.0".to_string(),
            }),
            previous_context: Some(Arc::new(BuildContext {
                reference: ComponentReference {
                    id: "child".to_string(),
                    version: "1.0.0".to_string(),
                },
                resolved_reference: OnceLock::new(),
                previous_context: Some(Arc::new(BuildContext {
                    reference: ComponentReference {
                        id: "child2".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    resolved_reference: OnceLock::from(ComponentReference {
                        id: "child2".to_string(),
                        version: "1.0.0".to_string(),
                    }),
                    previous_context: None,
                })),
            })),
        };

        assert!(context.contains_resolved_id("my-component"));
        assert!(context.contains_resolved_id("child2"));

        assert!(!context.contains_resolved_id("child"));
        assert!(!context.contains_resolved_id(ComponentReference::ROOT_ID));
    }
}
