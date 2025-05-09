use crate::json_editor::JsonEditorImpl;
use std::{io::Write, path::PathBuf, sync::Arc};

use anyhow::Context;
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, CallChain, Environment, Permissions, Rig,
    RigSession, RigSessionOptions, SlipwayReference, parse_rig,
};
use slipway_host::{
    render_state::{
        WriteComponentOutputs,
        to_view_model::{ComponentViewModel, RigExecutionStateViewModel},
    },
    run::RunEventHandler,
    tracing_writer::TraceOrWriter,
};

use crate::{
    canvas::render_canvas_if_exists, component_runners::get_component_runners,
    host_error::HostError,
};

#[allow(clippy::too_many_arguments)] // For now at least.
pub(super) async fn run_rig_from_component_file(
    mut w: Box<dyn Write>,
    component_reference: SlipwayReference,
    input: Option<String>,
    input_path: Option<std::path::PathBuf>,
    component_permissions: Permissions<'_>,
    registry_urls: Vec<String>,
    save_path: Option<PathBuf>,
    fonts_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let json_editor = JsonEditorImpl::new();
    let initial_input =
        super::debug_rig::get_component_input(&mut w, input, input_path, &json_editor)?;
    let rig = super::debug_rig::get_component_rig(
        component_reference,
        &component_permissions,
        initial_input,
    );

    // We created the rig, so we can trust it to only pass on the user specified
    // component_permissions to the component.
    // At minimum we would need component_permissions plus permission to load the
    // component, but there is no advantage to being more restrictive here.
    let rig_permissions = Permissions::allow_all();

    run_rig_inner(
        w,
        rig,
        rig_permissions,
        registry_urls,
        save_path,
        None,
        fonts_path,
    )
    .await
}

pub(super) async fn run_rig(
    mut w: Box<dyn Write>,
    input: std::path::PathBuf,
    engine_permissions: Permissions<'_>,
    registry_urls: Vec<String>,
    save_path: Option<PathBuf>,
    debug_rig_path: Option<PathBuf>,
    fonts_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    writeln!(&mut w, "Launching {}", input.display())?;
    let file_contents = tokio::fs::read_to_string(input.clone())
        .await
        .with_context(|| format!("Failed to read component from {}", input.display()))?;
    let rig = parse_rig(&file_contents)?;
    run_rig_inner(
        w,
        rig,
        engine_permissions,
        registry_urls,
        save_path,
        debug_rig_path,
        fonts_path,
    )
    .await
}

pub(super) async fn run_rig_inner(
    w: Box<dyn Write>,
    rig: Rig,
    engine_permissions: Permissions<'_>,
    registry_urls: Vec<String>,
    save_path: Option<PathBuf>,
    debug_rig_path: Option<PathBuf>,
    fonts_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let components_loader = BasicComponentsLoader::builder()
        .registry_lookup_urls(registry_urls)
        .build();

    let timezone = iana_time_zone::get_timezone()?;
    let locale = sys_locale::get_locale().unwrap_or(crate::DEFAULT_LOCALE.to_string());
    let component_cache = BasicComponentCache::primed(&rig, &components_loader).await?;
    let session_options = RigSessionOptions::new_for_run(
        debug_rig_path.is_some(),
        fonts_path.as_deref(),
        Environment { timezone, locale },
    )
    .await;
    let session = RigSession::new_with_options(rig, &component_cache, session_options);

    let mut event_handler = CliRunEventHandler::new(
        save_path,
        WriteComponentOutputsType::LeafComponents,
        TraceOrWriter::Writer(w),
    );
    let component_runners = get_component_runners();
    let component_runners_slice = component_runners.as_slice();

    let call_chain = Arc::new(CallChain::new(engine_permissions));

    let maybe_run_rig_result = slipway_host::run::run_rig(
        &session,
        &mut event_handler,
        component_runners_slice,
        call_chain,
    )
    .await;

    if let Some(debug_rig_path) = debug_rig_path {
        let debug_rig = session.run_record_as_rig();
        let debug_rig_json =
            serde_json::to_string_pretty(&debug_rig).context("Failed to serialize debug rig")?;
        tokio::fs::write(debug_rig_path, debug_rig_json)
            .await
            .context("Failed to write debug rig")?;
    }

    maybe_run_rig_result?;

    Ok(())
}

pub(super) struct CliRunEventHandler {
    save_path: Option<PathBuf>,
    write_outputs_type: WriteComponentOutputsType,
    inner: slipway_host::run::tracing_run_event_handler::TracingRunEventHandler,
}

impl CliRunEventHandler {
    pub fn new(
        save_path: Option<PathBuf>,
        write_outputs_type: WriteComponentOutputsType,
        level: TraceOrWriter,
    ) -> Self {
        Self {
            save_path,
            write_outputs_type,
            inner: slipway_host::run::tracing_run_event_handler::TracingRunEventHandler::new_for(
                level,
            ),
        }
    }
}

impl<'rig, 'cache> RunEventHandler<'rig, 'cache, HostError> for CliRunEventHandler {
    fn handle_component_run_start<'state>(
        &mut self,
        event: slipway_host::run::ComponentRunStartEvent<'rig>,
    ) -> Result<(), HostError> {
        self.inner
            .handle_component_run_start(event)
            .map_err(HostError::from)
    }

    fn handle_component_run_end(
        &mut self,
        event: slipway_host::run::ComponentRunEndEvent<'rig>,
    ) -> Result<(), HostError> {
        self.inner
            .handle_component_run_end(event)
            .map_err(HostError::from)
    }

    fn handle_state_changed<'state>(
        &mut self,
        event: slipway_host::run::StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<RigExecutionStateViewModel<'state>, HostError> {
        let is_complete = event.is_complete;

        let view_model = self
            .inner
            .handle_state_changed(event)
            .map_err(HostError::from)?;

        if is_complete {
            let write_component_outputs = SlipwayWriteComponentsOutputs {
                write_outputs_type: self.write_outputs_type,
            };

            write_component_outputs.write_component_outputs(
                self.inner.writer(),
                self.save_path.as_ref(),
                &view_model,
            )?;
        }

        Ok(view_model)
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
        view_model: &slipway_host::render_state::to_view_model::RigExecutionStateViewModel,
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
