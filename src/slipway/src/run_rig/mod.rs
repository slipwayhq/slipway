use std::{io::Write, path::PathBuf, sync::Arc};

use anyhow::Context;
use slipway_engine::{
    parse_rig, BasicComponentCache, BasicComponentsLoader, CallChain, Permissions, RigSession,
};
use slipway_host::run::RunEventHandler;

use crate::{
    component_runners::get_component_runners,
    host_error::HostError,
    render_state::{write_state, write_state_with_outputs},
};

pub(super) async fn run_rig<W: Write>(
    w: &mut W,
    input: std::path::PathBuf,
    engine_permissions: Permissions<'_>,
    registry_urls: Vec<String>,
    save_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    writeln!(w, "Launching {}", input.display())?;
    writeln!(w)?;

    let file_contents = std::fs::read_to_string(input.clone())
        .with_context(|| format!("Failed to read component from {}", input.display()))?;
    let rig = parse_rig(&file_contents)?;

    let components_loader = BasicComponentsLoader::builder()
        .registry_lookup_urls(registry_urls)
        .build();

    let component_cache = BasicComponentCache::primed(&rig, &components_loader)?;
    let session = RigSession::new(rig, &component_cache);

    let mut event_handler = SlipwayRunEventHandler { w, save_path };
    let component_runners = get_component_runners();
    let component_runners_slice = component_runners.as_slice();

    let call_chain = Arc::new(CallChain::new(engine_permissions));

    slipway_host::run::run_rig(
        &session,
        &mut event_handler,
        component_runners_slice,
        call_chain,
    )
    .await?;

    Ok(())
}

struct SlipwayRunEventHandler<'w, W: Write> {
    w: &'w mut W,
    save_path: Option<PathBuf>,
}

impl<'rig, 'cache, W: Write> RunEventHandler<'rig, 'cache, HostError>
    for SlipwayRunEventHandler<'_, W>
{
    fn handle_component_run_start(
        &mut self,
        event: slipway_host::run::ComponentRunStartEvent<'rig>,
    ) -> Result<(), HostError> {
        writeln!(self.w, r#"Running "{}"..."#, event.component_handle)?;
        Ok(())
    }

    fn handle_component_run_end(
        &mut self,
        _event: slipway_host::run::ComponentRunEndEvent<'rig>,
    ) -> Result<(), HostError> {
        writeln!(self.w)?;
        Ok(())
    }

    fn handle_state_changed<'state>(
        &mut self,
        event: slipway_host::run::StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<(), HostError> {
        if event.is_complete {
            writeln!(self.w, "No more components to run.")?;
            writeln!(self.w)?;
            write_state_with_outputs(
                self.w,
                self.save_path.as_ref(),
                event.state,
                crate::render_state::PrintComponentOutputsType::LeafComponents,
            )?;
        } else {
            write_state(self.w, event.state)?;
        }

        Ok(())
    }
}
