use std::rc::Rc;

use super::validation_context::ValidationContext;

#[derive(Debug)]
pub enum ValidationFailure {
    Error(String, Rc<ValidationContext>),
    Warning(String, Rc<ValidationContext>),
}

impl std::fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationFailure::Error(message, context) => {
                write!(f, "Error: {} ({})", message, context.get_path())
            }
            ValidationFailure::Warning(message, context) => {
                write!(f, "Warning: {} ({})", message, context.get_path())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_render_friendly_message() {
        let context = Rc::new(ValidationContext {
            node_name: "inputs".to_string(),
            previous_context: Some(Rc::new(ValidationContext {
                node_name: "component".to_string(),
                previous_context: None,
            })),
        });

        let failure = ValidationFailure::Error("Error message".to_string(), Rc::clone(&context));
        assert_eq!(
            failure.to_string(),
            "Error: Error message (component.inputs)"
        );

        let failure =
            ValidationFailure::Warning("Warning message".to_string(), Rc::clone(&context));
        assert_eq!(
            failure.to_string(),
            "Warning: Warning message (component.inputs)"
        );
    }
}
