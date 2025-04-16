use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::lock::Mutex;

use crate::errors::RigError;
use crate::load::ComponentCache;
use crate::{CallChain, Callout, ComponentHandle, ComponentInput, Immutable, SlipwayReference};

use super::fonts::FontContext;
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
        callouts: Option<HashMap<ComponentHandle, Callout>>,
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
    font_context: Arc<Mutex<FontContext>>,
}

impl RigSessionOptions {
    pub async fn new_for_serve(
        base_path: PathBuf,
        aot_path: Option<PathBuf>,
        fonts_path: PathBuf,
    ) -> Self {
        let font_context = FontContext::new_with_path(&fonts_path).await;
        RigSessionOptions {
            base_path,
            aot_path,
            run_record: None,
            font_context: Arc::new(Mutex::new(font_context)),
        }
    }

    pub async fn new_for_run(use_run_record: bool, fonts_path: Option<&Path>) -> Self {
        let run_record = if use_run_record {
            Some(RigRunRecord::new())
        } else {
            None
        };

        let font_context = match fonts_path {
            None => FontContext::new(),
            Some(fonts_path) => FontContext::new_with_path(fonts_path).await,
        };

        RigSessionOptions {
            base_path: PathBuf::from("."),
            aot_path: None,
            run_record,
            font_context: Arc::new(Mutex::new(font_context)),
        }
    }

    pub fn font_context(&self) -> Arc<Mutex<FontContext>> {
        Arc::clone(&self.font_context)
    }
}
