use std::{io::Write, path::PathBuf, sync::Arc};

use anyhow::Context;
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, CallChain, Permissions, RigSession, parse_rig,
};
use slipway_host::{
    render_state::to_view_model::ComponentViewModel,
    render_state::{WriteComponentOutputs, write_state, write_state_with_outputs},
    run::RunEventHandler,
};

use crate::{
    canvas::render_canvas_if_exists, component_runners::get_component_runners,
    host_error::HostError,
};

pub(super) async fn run_rig<W: Write>(
    w: &mut W,
    input: std::path::PathBuf,
    engine_permissions: Permissions<'_>,
    registry_urls: Vec<String>,
    save_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    writeln!(w, "Launching {}", input.display())?;

    let file_contents = tokio::fs::read_to_string(input.clone())
        .await
        .with_context(|| format!("Failed to read component from {}", input.display()))?;
    let rig = parse_rig(&file_contents)?;

    let components_loader = BasicComponentsLoader::builder()
        .registry_lookup_urls(registry_urls)
        .build();

    let component_cache = BasicComponentCache::primed(&rig, &components_loader).await?;
    let session = RigSession::new(rig, &component_cache);

    let mut event_handler =
        CliRunEventHandler::new(w, save_path, WriteComponentOutputsType::LeafComponents);
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

pub(super) struct CliRunEventHandler<'w, W: Write> {
    w: &'w mut W,
    save_path: Option<PathBuf>,
    write_outputs_type: WriteComponentOutputsType,
}

impl<'w, W: Write> CliRunEventHandler<'w, W> {
    pub fn new(
        w: &'w mut W,
        save_path: Option<PathBuf>,
        write_outputs_type: WriteComponentOutputsType,
    ) -> Self {
        Self {
            w,
            save_path,
            write_outputs_type,
        }
    }
}

impl<'rig, 'cache, W: Write> RunEventHandler<'rig, 'cache, HostError>
    for CliRunEventHandler<'_, W>
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
            write_state_with_outputs(
                self.w,
                self.save_path.as_ref(),
                event.state,
                SlipwayWriteComponentsOutputs {
                    write_outputs_type: self.write_outputs_type,
                },
            )?;
        } else {
            write_state::<_, HostError>(self.w, event.state)?;
        }

        Ok(())
    }
}

struct SlipwayWriteComponentsOutputs {
    write_outputs_type: WriteComponentOutputsType,
}

impl<W: Write> WriteComponentOutputs<W, HostError> for SlipwayWriteComponentsOutputs {
    fn write_component_outputs(
        &self,
        w: &mut W,
        save_path: Option<&PathBuf>,
        view_model: slipway_host::render_state::to_view_model::RigExecutionStateViewModel,
    ) -> Result<(), HostError> {
        match self.write_outputs_type {
            WriteComponentOutputsType::None => {}
            WriteComponentOutputsType::LeafComponents => {
                for group in view_model.groups.iter() {
                    for component in group.components.iter() {
                        if !component.output_row_indexes.is_empty() {
                            continue;
                        }

                        write_component_output(w, save_path, component)?;
                    }
                }
            }
            WriteComponentOutputsType::AllComponents => {
                for group in view_model.groups.iter() {
                    for component in group.components.iter() {
                        write_component_output(w, save_path, component)?;
                    }
                }
            }
        };

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum WriteComponentOutputsType {
    None,
    LeafComponents,
    AllComponents,
}

fn write_component_output<W: Write>(
    w: &mut W,
    save_path: Option<&PathBuf>,
    component: &ComponentViewModel,
) -> Result<(), HostError> {
    if let Some(save_path) = save_path.as_ref() {
        std::fs::create_dir_all(save_path).map_err(|error| {
            HostError::Other(format!(
                "Failed to create directory to save outputs: {}",
                error
            ))
        })?;
    }

    if let Some(output) = component.state.output() {
        writeln!(w, r#"Component "{}" output:"#, component.handle)?;

        if !render_canvas_if_exists(
            component.handle,
            output,
            save_path.map(|p| p.join(format!("{}.png", component.handle.0))),
        )? {
            writeln!(w, "{:#}", output)?;

            if let Some(save_path) = save_path {
                let output_path = save_path.join(format!("{}.json", component.handle.0));
                let output_file = std::fs::File::create(output_path).map_err(|error| {
                    HostError::Other(format!(
                        "Failed to create output file for component {}: {}",
                        component.handle, error
                    ))
                })?;

                serde_json::to_writer_pretty(output_file, output).map_err(|error| {
                    HostError::Other(format!(
                        "Failed to write output file for component {}: {}",
                        component.handle, error
                    ))
                })?;
            }
        }

        writeln!(w)?;
    }

    Ok(())
}
