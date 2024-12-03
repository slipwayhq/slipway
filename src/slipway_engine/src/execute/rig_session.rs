use crate::errors::RigError;
use crate::load::ComponentCache;
use crate::Immutable;

use super::initialize::initialize;
use super::rig_execution_state::RigExecutionState;

use crate::parse::types::Rig;

pub struct RigSession<'cache> {
    pub(crate) rig: Rig,
    pub(crate) component_cache: &'cache ComponentCache,
    pub(crate) options: RigSessionOptions,
}

impl<'cache> RigSession<'cache> {
    pub fn new_with_options(
        rig: Rig,
        component_cache: &'cache ComponentCache,
        options: RigSessionOptions,
    ) -> Self {
        RigSession {
            rig,
            component_cache,
            options,
        }
    }

    pub fn new(rig: Rig, component_cache: &'cache ComponentCache) -> Self {
        RigSession {
            rig,
            component_cache,
            options: Default::default(),
        }
    }

    pub fn initialize<'rig>(
        &'rig self,
    ) -> Result<Immutable<RigExecutionState<'rig, 'cache>>, RigError> {
        initialize(self)
    }
}

#[derive(Default)]
pub struct RigSessionOptions {}
