use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::errors::RigError;
use crate::load::ComponentCache;
use crate::{CallChain, ComponentHandle, ComponentInput, Immutable, SlipwayReference};

use super::initialize::initialize;
use super::rig_execution_state::RigExecutionState;
use super::run_record::RigRunRecord;

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

    pub fn push_run_record(
        &self,
        component_reference: SlipwayReference,
        call_chain: Arc<CallChain>,
        input: Arc<ComponentInput>,
        callouts: Option<HashMap<ComponentHandle, SlipwayReference>>,
    ) {
        if let Some(run_record) = &self.options.run_record {
            run_record.push_run_record(component_reference, call_chain, input, callouts);
        }
    }

    pub fn run_record_enabled(&self) -> bool {
        self.options.run_record.is_some()
    }

    pub fn run_record_as_rig(&self) -> Rig {
        let Some(run_record) = &self.options.run_record else {
            panic!("run record should exist");
        };

        run_record.run_record_as_rig()
    }
}

#[derive(Default, Debug, Clone)]
pub struct RigSessionOptions {
    pub base_path: PathBuf,
    pub aot_path: Option<PathBuf>,
    run_record: Option<RigRunRecord>,
}

impl RigSessionOptions {
    pub fn new_for_serve(base_path: PathBuf, aot_path: Option<PathBuf>) -> Self {
        RigSessionOptions {
            base_path,
            aot_path,
            run_record: None,
        }
    }

    pub fn new_for_run(use_run_record: bool) -> Self {
        let run_record = if use_run_record {
            Some(RigRunRecord::new())
        } else {
            None
        };

        RigSessionOptions {
            base_path: PathBuf::from("."),
            aot_path: None,
            run_record,
        }
    }
}
