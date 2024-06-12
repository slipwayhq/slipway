use crate::errors::AppError;
use crate::load::ComponentCache;
use crate::Immutable;

use super::app_execution_state::AppExecutionState;
use super::initialize::initialize;

use crate::parse::types::App;

pub struct AppSession {
    pub(crate) app: App,
    pub(crate) component_cache: ComponentCache,
    pub(crate) options: AppSessionOptions,
}

impl AppSession {
    pub fn new_with_options(
        app: App,
        component_cache: ComponentCache,
        options: AppSessionOptions,
    ) -> Self {
        AppSession {
            app,
            component_cache,
            options,
        }
    }

    pub fn new(app: App, component_cache: ComponentCache) -> Self {
        AppSession {
            app,
            component_cache,
            options: Default::default(),
        }
    }
}

impl AppSession {
    pub fn initialize(&self) -> Result<Immutable<AppExecutionState>, AppError> {
        initialize(self)
    }
}

#[derive(Default)]
pub struct AppSessionOptions {}
