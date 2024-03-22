use std::sync::Arc;

use crate::{errors::SlipwayError, SlipwayReference};

use super::{component_loaders::ComponentLoader, primitives::LoaderId};

pub(crate) struct LoadedComponent<T> {
    component_reference: SlipwayReference,
    value: Option<T>,
    loader_failures: Vec<LoaderFailure>,
}

pub(crate) struct LoaderFailure {
    loader_id: LoaderId,
    error: SlipwayError,
}

pub(super) async fn load_component<T>(
    component_reference: SlipwayReference,
    loaders: Arc<Vec<Box<dyn ComponentLoader<T>>>>,
) -> LoadedComponent<T> {
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
                errors.push(LoaderFailure {
                    loader_id: loader.id(),
                    error: e,
                });
            }
        }
    }

    LoadedComponent::<T> {
        component_reference: component_reference.clone(),
        value: loaded_component,
        loader_failures: errors,
    }
}
