use std::path::PathBuf;

use crate::errors::RigError;
use crate::load::ComponentCache;
use crate::{Immutable, SlipwayReference};

use super::initialize::initialize;
use super::rig_execution_state::RigExecutionState;

use crate::parse::types::Rig;

pub struct RigSession<'cache> {
    pub(crate) rig: Rig,
    pub(crate) component_cache: &'cache dyn ComponentCache,
    pub(crate) options: RigSessionOptions,
}

impl<'cache> RigSession<'cache> {
    pub fn new_with_options(
        rig: Rig,
        component_cache: &'cache dyn ComponentCache,
        options: RigSessionOptions,
    ) -> Self {
        RigSession {
            rig,
            component_cache,
            options,
        }
    }

    pub fn new(rig: Rig, component_cache: &'cache dyn ComponentCache) -> Self {
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

    pub fn rigging_component_references(&self) -> Vec<&SlipwayReference> {
        self.rig
            .rigging
            .components
            .values()
            .map(|c| &c.component)
            .collect()
    }
}

#[derive(Default)]
pub struct RigSessionOptions {
    pub base_path: PathBuf,
}

impl RigSessionOptions {
    pub fn new(base_path: PathBuf) -> Self {
        RigSessionOptions { base_path }
    }
}
