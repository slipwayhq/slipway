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
}

impl AppSession {
    pub fn new(app: App) -> Self {
        AppSession {
            app,
            component_cache: RefCell::new(Box::new(InMemoryComponentCache::new(
                vec![Box::new(LocalComponentLoader {})],
                vec![Box::new(LocalComponentLoader {})],
            ))),
        }
    }
}

impl AppSession {
    pub fn initialize(&self) -> Result<Immutable<AppExecutionState>, AppError> {
        initialize(self)
    }
}

pub(crate) struct AppSessionOptions {
    // pub(crate) execution_concurrency: usize,
}
