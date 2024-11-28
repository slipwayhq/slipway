use crate::errors::RigError;
use crate::load::ComponentCache;
use crate::Immutable;

use super::rig_execution_state::RigExecutionState;
use super::initialize::initialize;

use crate::parse::types::Rig;

pub struct RigSession {
    pub(crate) rig: Rig,
    pub(crate) component_cache: ComponentCache,
    pub(crate) options: RigSessionOptions,
}

impl RigSession {
    pub fn new_with_options(
        rig: Rig,
        component_cache: ComponentCache,
        options: RigSessionOptions,
    ) -> Self {
        RigSession {
            rig,
            component_cache,
            options,
        }
    }

    pub fn new(rig: Rig, component_cache: ComponentCache) -> Self {
        RigSession {
            rig,
            component_cache,
            options: Default::default(),
        }
    }
}

impl RigSession {
    pub fn initialize(&self) -> Result<Immutable<RigExecutionState>, RigError> {
        initialize(self)
    }
}

#[derive(Default)]
pub struct RigSessionOptions {}
