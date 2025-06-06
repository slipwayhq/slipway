use std::sync::Arc;

use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, CallChain, ComponentHandle, Environment,
    Permission, Rig, RigSession, RigSessionOptions,
};
use slipway_host::tracing_writer::TraceOrWriter;

use crate::{
    component_runners::get_component_runners,
    permissions::PERMISSIONS_EMPTY,
    primitives::RigName,
    run_rig::{CliRunEventHandler, WriteComponentOutputsType},
    serve::repository::file_system::FONTS_FOLDER_NAME,
};

use super::super::ServeState;

pub async fn run_rig(
    state: Arc<ServeState>,
    rig: Rig,
    rig_name: &RigName,
    device_context: Option<serde_json::Value>,
) -> anyhow::Result<RunRigResult> {
    let components_loader = BasicComponentsLoader::builder()
        .local_base_directory(&state.base_path)
        .registry_lookup_urls(state.config.registry_urls.clone())
        .build();

    let timezone = state
        .config
        .environment
        .timezone
        .as_ref()
        .map(|tz| tz.name().to_string())
        .unwrap_or_else(crate::utils::get_system_timezone);

    let locale = state
        .config
        .environment
        .locale
        .clone()
        .unwrap_or_else(crate::utils::get_system_timezone);

    let component_cache = BasicComponentCache::primed(&rig, &components_loader).await?;
    let session_options = RigSessionOptions::new_for_serve(
        &rig,
        state.base_path.clone(),
        state.aot_path.clone(),
        state.base_path.join(FONTS_FOLDER_NAME),
        Environment { timezone, locale },
        device_context,
    )
    .await;
    let session = RigSession::new_with_options(rig, &component_cache, session_options);

    let mut event_handler = CliRunEventHandler::new(
        None,
        WriteComponentOutputsType::None,
        TraceOrWriter::Trace(tracing::Level::INFO),
    );

    let component_runners = get_component_runners();
    let component_runners_slice = component_runners.as_slice();

    let rig_permissions = state
        .config
        .rig_permissions
        .get(rig_name)
        .unwrap_or_else(|| &PERMISSIONS_EMPTY);

    let call_chain = Arc::new(CallChain::new(rig_permissions.into()));

    let result = slipway_host::run::run_rig(
        &session,
        &mut event_handler,
        component_runners_slice,
        call_chain,
    )
    .await?;

    let rig_output = crate::get_rig_output::get_rig_output(&result)?;

    Ok(RunRigResult {
        handle: rig_output.handle.clone(),
        output: rig_output.output.value.clone(),
    })
}

#[derive(Debug, Default, serde::Deserialize)]
struct RigConfig {
    #[serde(default)]
    allow: Vec<Permission>,

    #[serde(default)]
    deny: Vec<Permission>,
}

pub struct RunRigResult {
    pub handle: ComponentHandle,
    pub output: serde_json::Value,
}
