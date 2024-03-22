use std::str::FromStr;

use async_trait::async_trait;

use crate::{errors::SlipwayError, Component, SlipwayReference};

use super::primitives::LoaderId;

#[async_trait]
pub(crate) trait ComponentLoader<TResult>: Send + Sync {
    fn id(&self) -> LoaderId;

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<TResult>, SlipwayError>;
}

struct LocalComponentLoader {}

#[async_trait]
impl ComponentLoader<Component> for LocalComponentLoader {
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
impl ComponentLoader<Vec<u8>> for LocalComponentLoader {
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
