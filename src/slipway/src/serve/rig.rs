use std::{str::FromStr, sync::Arc};

use anyhow::Context;
use slipway_engine::{
    parse_rig_json, BasicComponentCache, BasicComponentsLoader, CallChain, ComponentHandle,
    Permission, Permissions, RigSession,
};
use tracing::info;

use crate::{component_runners::get_component_runners, run_rig::SlipwayRunEventHandler};

use super::ServeState;

pub(super) async fn run_rig(
    state: Arc<ServeState>,
    rig_name: &str,
    rig_json: serde_json::Value,
) -> anyhow::Result<RunRigResult> {
    let config_path = state.root.join(format!("{rig_name}.config.json"));
    let config = match std::fs::File::open(&config_path) {
        Ok(file) => {
            serde_json::from_reader(file).context("Failed to parse Slipway Serve config file.")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => RigConfig::default(),
        Err(e) => return Err(e).context("Failed to load Slipway Serve config file.")?,
    };

    let rig = parse_rig_json(rig_json)?;

    let components_loader = BasicComponentsLoader::builder()
        .registry_lookup_urls(state.config.registry_urls.clone())
        .build();

    let component_cache = BasicComponentCache::primed(&rig, &components_loader)?;
    let session = RigSession::new(rig, &component_cache);

    let mut writer = TracingWriter::new();

    let mut event_handler = SlipwayRunEventHandler::new(&mut writer, None);
    let component_runners = get_component_runners();
    let component_runners_slice = component_runners.as_slice();

    let allow = state
        .config
        .allow
        .iter()
        .chain(config.allow.iter())
        .cloned()
        .collect();

    let deny = state
        .config
        .deny
        .iter()
        .chain(config.deny.iter())
        .cloned()
        .collect();

    let engine_permissions = Permissions::new(&allow, &deny);
    let call_chain = Arc::new(CallChain::new(engine_permissions));

    let result = slipway_host::run::run_rig(
        &session,
        &mut event_handler,
        component_runners_slice,
        call_chain,
    )
    .await?;

    result
        .component_outputs
        .get(&ComponentHandle::from_str("render").unwrap())
        .as_ref()
        .map_or_else(
            || Err(anyhow::anyhow!("No output from rig")),
            |output| {
                Ok(RunRigResult {
                    handle: ComponentHandle::from_str("render").unwrap(),
                    output: output.as_ref().unwrap().value.clone(),
                })
            },
        )
}

#[derive(Debug, Default, serde::Deserialize)]
struct RigConfig {
    #[serde(default)]
    allow: Vec<Permission>,

    #[serde(default)]
    deny: Vec<Permission>,
}

pub(super) struct RunRigResult {
    pub handle: ComponentHandle,
    pub output: serde_json::Value,
}

#[derive(Debug)]
struct TracingWriter {
    buffer: String,
}

impl TracingWriter {
    fn new() -> Self {
        TracingWriter {
            buffer: String::new(),
        }
    }
}

impl std::io::Write for TracingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.buffer.push_str(s);
                while let Some(idx) = self.buffer.find('\n') {
                    let line = self.buffer.drain(..=idx).collect::<String>();
                    info!("{}", line.trim_end_matches('\n'));
                }
            }
            Err(_) => {
                // Fallback for non-UTF8 data
                self.buffer.push_str(&format!("{:?}", buf));
                while let Some(idx) = self.buffer.find('\n') {
                    let line = self.buffer.drain(..=idx).collect::<String>();
                    info!("{}", line.trim_end_matches('\n'));
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            info!("{}", self.buffer);
            self.buffer.clear();
        }
        Ok(())
    }
}
