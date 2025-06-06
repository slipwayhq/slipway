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

pub const TEST_TIMEZONE: &str = "Canada/Eastern";
pub const TEST_LOCALE: &str = "fr-CA";

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

    pub fn new_for_test(rig: Rig, component_cache: &'cache dyn ComponentCache) -> Self {
        let options = RigSessionOptions::new_for_test(
            &rig,
            Environment {
                timezone: TEST_TIMEZONE.to_string(),
                locale: TEST_LOCALE.to_string(),
            },
            Some(serde_json::json!({
                "width": 800,
                "height": 480,
            })),
        );

        RigSession {
            rig,
            component_cache,
            options,
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

#[derive(Debug, Clone)]
pub struct RigSessionOptions {
    pub base_path: PathBuf,
    pub aot_path: Option<PathBuf>,
    pub environment: Environment,
    pub rig_additional_context: serde_json::Value,
    run_record: Option<RigRunRecord>,
    font_context: Arc<Mutex<FontContext>>,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub timezone: String,
    pub locale: String,
}

impl Environment {
    pub fn for_test() -> Self {
        Environment {
            timezone: TEST_TIMEZONE.to_string(),
            locale: TEST_LOCALE.to_string(),
        }
    }
}

impl RigSessionOptions {
    pub async fn new_for_serve(
        rig: &Rig,
        base_path: PathBuf,
        aot_path: Option<PathBuf>,
        fonts_path: PathBuf,
        environment: Environment,
        device_context: Option<serde_json::Value>,
    ) -> Self {
        let font_context = FontContext::new_with_path(&fonts_path).await;
        let device_context = device_context.or_else(|| rig.context.clone().and_then(|c| c.device));
        let rig_additional_context = get_rig_additional_context(&environment, device_context);

        RigSessionOptions {
            base_path,
            aot_path,
            environment,
            rig_additional_context,
            run_record: None,
            font_context: Arc::new(Mutex::new(font_context)),
        }
    }

    pub async fn new_for_run(
        rig: &Rig,
        use_run_record: bool,
        fonts_path: Option<&Path>,
        environment: Environment,
    ) -> Self {
        let run_record = if use_run_record {
            Some(RigRunRecord::new())
        } else {
            None
        };

        let font_context = match fonts_path {
            None => FontContext::new(),
            Some(fonts_path) => FontContext::new_with_path(fonts_path).await,
        };

        let device_context = rig.context.clone().and_then(|c| c.device);
        let rig_additional_context = get_rig_additional_context(&environment, device_context);

        RigSessionOptions {
            base_path: PathBuf::from("."),
            aot_path: None,
            environment,
            rig_additional_context,
            run_record,
            font_context: Arc::new(Mutex::new(font_context)),
        }
    }

    pub fn new_for_test(
        rig: &Rig,
        environment: Environment,
        device_context: Option<serde_json::Value>,
    ) -> Self {
        let device_context = device_context.or_else(|| rig.context.clone().and_then(|c| c.device));
        let rig_additional_context = get_rig_additional_context(&environment, device_context);

        RigSessionOptions {
            base_path: PathBuf::from("."),
            aot_path: None,
            environment,
            rig_additional_context,
            run_record: None,
            font_context: Arc::new(Mutex::new(FontContext::new())),
        }
    }

    pub fn font_context(&self) -> Arc<Mutex<FontContext>> {
        Arc::clone(&self.font_context)
    }
}

fn get_rig_additional_context(
    environment: &Environment,
    device_context: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "timezone": environment.timezone,
        "locale": environment.locale,
        "device": device_context,
    })
}
