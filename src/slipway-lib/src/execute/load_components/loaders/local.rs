use async_trait::async_trait;

use std::str::FromStr;

use super::ComponentPartLoader;

use crate::{
    errors::SlipwayError, execute::load_components::primitives::LoaderId, Component,
    SlipwayReference,
};

pub(crate) struct LocalComponentLoader {}

#[async_trait]
impl ComponentPartLoader<Component> for LocalComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("local").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<Component>, SlipwayError> {
        match component_reference {
            SlipwayReference::Local { path } => {
                let file_contents = std::fs::read_to_string(path).map_err(|e| {
                    SlipwayError::ComponentLoadFailed(
                        path.to_string_lossy().to_string(),
                        e.to_string(),
                    )
                })?;
                let component = crate::parse::parse_component(&file_contents)?;
                Ok(Some(component))
            }
            _ => Ok(None),
        }
    }
}

#[async_trait]
impl ComponentPartLoader<Vec<u8>> for LocalComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("local").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<Vec<u8>>, SlipwayError> {
        match component_reference {
            SlipwayReference::Local { path } => {
                let path = path.with_extension("wasm");
                let file_contents = std::fs::read(&path).map_err(|e| {
                    SlipwayError::ComponentLoadFailed(
                        path.to_string_lossy().to_string(),
                        e.to_string(),
                    )
                })?;
                Ok(Some(file_contents))
            }
            _ => Ok(None),
        }
    }
}
