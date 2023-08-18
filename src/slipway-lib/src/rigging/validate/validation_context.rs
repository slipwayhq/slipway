use std::rc::Rc;

#[derive(Debug)]
pub struct ValidationContext {
    pub node_name: String,
    pub previous_context: Option<Rc<ValidationContext>>,
}

impl ValidationContext {
    pub fn get_path(&self) -> String {
        let mut path = self.node_name.clone();

        let mut current_context = self.previous_context.clone();
        while let Some(context) = current_context {
            path.insert(0, '.');
            path.insert_str(0, &context.node_name);
            current_context = context.previous_context.clone();
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_context_path() {
        let context = Rc::new(ValidationContext {
            node_name: "inputs".to_string(),
            previous_context: Some(Rc::new(ValidationContext {
                node_name: "component".to_string(),
                previous_context: None,
            })),
        });

        assert_eq!(context.get_path(), "component.inputs");
    }
}
