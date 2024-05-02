use crate::errors::AppError;
use crate::Immutable;

use super::app_execution_state::AppExecutionState;
use super::initialize::initialize;
use super::load_components::{InMemoryComponentCache, LoadedComponentCache, LocalComponentLoader};

use std::cell::RefCell;

use crate::parse::types::App;

pub struct AppSession {
    pub(crate) app: App,
    pub(crate) component_cache: RefCell<Box<dyn LoadedComponentCache>>,
    pub(crate) component_load_error_behavior: ComponentLoaderErrorBehavior,
}

impl AppSession {
    pub fn new(app: App, options: AppSessionOptions) -> Self {
        AppSession {
            app,
            component_cache: RefCell::new(Box::new(InMemoryComponentCache::new(
                vec![Box::new(LocalComponentLoader {})],
                vec![Box::new(LocalComponentLoader {})],
            ))),
            component_load_error_behavior: options.component_load_error_behavior,
        }
    }
}

impl AppSession {
    pub fn initialize(&self) -> Result<Immutable<AppExecutionState>, AppError> {
        initialize(self)
    }
}

pub struct AppSessionOptions {
    pub(crate) component_load_error_behavior: ComponentLoaderErrorBehavior,
}

pub enum ComponentLoaderErrorBehavior {
    ErrorAlways,
    ErrorIfComponentNotLoaded,
}

impl Default for AppSessionOptions {
    fn default() -> Self {
        AppSessionOptions {
            component_load_error_behavior: ComponentLoaderErrorBehavior::ErrorAlways,
        }
    }
}
