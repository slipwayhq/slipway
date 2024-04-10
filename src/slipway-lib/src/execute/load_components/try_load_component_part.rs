use std::sync::Arc;

use crate::{
    errors::{ComponentLoadError, ComponentLoaderFailure},
    SlipwayReference,
};

use super::loaders::ComponentPartLoader;

pub(crate) struct LoadComponentResult<T> {
    pub component_reference: SlipwayReference,
    pub value: Option<T>,
    pub loader_failures: Vec<ComponentLoaderFailure>,
}

pub(super) async fn try_load_component_part<T>(
    component_reference: SlipwayReference,
    loaders: Arc<Vec<Box<dyn ComponentPartLoader<T>>>>,
) -> LoadComponentResult<T> {
    let mut loaded_component = None;
    let mut errors = Vec::new();

    for loader in loaders.iter() {
        match loader.load(&component_reference).await {
            Ok(Some(component)) => {
                loaded_component = Some(component);
                break;
            }
            Ok(None) => {}
            Err(e) => {
                errors.push(ComponentLoaderFailure {
                    loader_id: Some(loader.id()),
                    error: e,
                });
            }
        }
    }

    if errors.is_empty() && loaded_component.is_none() {
        let all_loader_ids = loaders.iter().map(|loader| loader.id()).collect();

        errors.push(ComponentLoaderFailure {
            loader_id: None,
            error: ComponentLoadError::NotFound {
                reference: component_reference.clone(),
                loader_ids: all_loader_ids,
            },
        });
    }

    LoadComponentResult::<T> {
        component_reference: component_reference.clone(),
        value: loaded_component,
        loader_failures: errors,
    }
}
