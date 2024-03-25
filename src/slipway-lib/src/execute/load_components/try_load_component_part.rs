use std::sync::Arc;

use crate::{errors::SlipwayError, SlipwayReference};

use super::{loaders::ComponentPartLoader, primitives::LoaderId};

pub(crate) struct LoadComponentResult<T> {
    component_reference: SlipwayReference,
    value: Option<T>,
    loader_failures: Vec<LoaderFailure>,
}

pub(crate) struct LoaderFailure {
    loader_id: LoaderId,
    error: SlipwayError,
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
                errors.push(LoaderFailure {
                    loader_id: loader.id(),
                    error: e,
                });
            }
        }
    }

    LoadComponentResult::<T> {
        component_reference: component_reference.clone(),
        value: loaded_component,
        loader_failures: errors,
    }
}
