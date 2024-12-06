use std::io::ErrorKind;

use super::errors::SlipwayDebugError;

pub(super) trait JsonEditor {
    fn edit(&self, template: &serde_json::Value) -> Result<serde_json::Value, SlipwayDebugError>;
}

pub(super) struct JsonEditorImpl;

impl JsonEditorImpl {
    pub(super) fn new() -> Self {
        Self
    }
}

impl JsonEditor for JsonEditorImpl {
    fn edit(&self, template: &serde_json::Value) -> Result<serde_json::Value, SlipwayDebugError> {
        let template_string = serde_json::to_string_pretty(&template)
            .expect("Component input should be serializable");
        let maybe_edited = edit::edit(template_string);
        match maybe_edited {
            Ok(edited) => {
                let result = serde_json::from_str(&edited)?;
                Ok(result)
            }
            Err(e) => match e.kind() {
                ErrorKind::InvalidData => Err(SlipwayDebugError::UserError(
                    "Could not decode input as UTF-8".into(),
                )),
                ErrorKind::NotFound => {
                    Err(SlipwayDebugError::UserError("Text editor not found".into()))
                }
                other_error => Err(SlipwayDebugError::UserError(format!(
                    "Failed to open the file: {:#?}",
                    other_error
                ))),
            },
        }
    }
}
