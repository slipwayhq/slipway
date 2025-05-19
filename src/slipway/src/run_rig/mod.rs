use crate::json_editor::JsonEditorImpl;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use slipway_engine::{
    BasicComponentCache, BasicComponentsLoader, CallChain, Environment, Immutable, Permissions,
    Rig, RigExecutionState, RigSession, RigSessionOptions, SlipwayReference, parse_rig,
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

    let timezone = crate::utils::get_system_timezone();
    let locale = crate::utils::get_system_locale();
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

        let state = event.state;
        let view_model = self
            .inner
            .handle_state_changed(event)
            .map_err(HostError::from)?;

        if is_complete {
            let write_component_outputs = SlipwayWriteMultipleComponentOutputs {
                write_outputs_type: self.write_outputs_type,
            };

            write_component_outputs.write_component_outputs(
                self.inner.writer(),
                self.save_path.as_deref(),
                state,
                &view_model,
            )?;
        }

        Ok(view_model)
    }
}

struct SlipwayWriteMultipleComponentOutputs {
    write_outputs_type: WriteComponentOutputsType,
}

impl<W: Write> WriteComponentOutputs<W, HostError> for SlipwayWriteMultipleComponentOutputs {
    fn write_component_outputs(
        &self,
        w: &mut W,
        save_path: Option<&Path>,
        state: &Immutable<RigExecutionState<'_, '_>>,
        view_model: &slipway_host::render_state::to_view_model::RigExecutionStateViewModel,
    ) -> Result<(), HostError> {
        let extension = save_path.and_then(|p| p.extension().and_then(|ext| ext.to_str()));
        match extension {
            Some(extension) => {
                let rig_output = crate::get_rig_output::get_rig_output(state)
                    .map_err(|e| HostError::Other(format!("{e}")))?;

                let save_path_unwrapped = save_path.unwrap();
                writeln!(
                    w,
                    "Writing \"{}\" output to: {}",
                    rig_output.handle,
                    save_path_unwrapped.to_string_lossy()
                )?;

                match extension {
                    "json" => {
                        let output_file =
                            std::fs::File::create(save_path_unwrapped).map_err(|error| {
                                HostError::Other(format!(
                                    "Failed to create output file for rig: {}",
                                    error
                                ))
                            })?;

                        serde_json::to_writer_pretty(output_file, &rig_output.output.value)
                            .map_err(|error| {
                                HostError::Other(format!(
                                    "Failed to write output file for rig: {}",
                                    error
                                ))
                            })?;
                    }
                    "png" => {
                        if !render_canvas_if_exists(
                            rig_output.handle,
                            &rig_output.output.value,
                            save_path,
                        )? {
                            return Err(HostError::Other(format!(
                                "No canvas found for rig output: {}",
                                rig_output.handle
                            )));
                        }
                    }
                    _ => {
                        return Err(HostError::Other(format!(
                            "File extension should be \"json\" or \"png\". Extension was \"{}\".",
                            extension
                        )));
                    }
                }
            }
            None => match self.write_outputs_type {
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
            },
        }

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
    save_path: Option<&Path>,
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
        if let Some(save_path) = save_path {
            writeln!(
                w,
                "Writing \"{}\" output to folder: {}",
                component.handle,
                save_path.to_string_lossy()
            )?;
        } else {
            writeln!(w, "Component \"{}\" output:", component.handle)?;
        }

        if !render_canvas_if_exists(
            component.handle,
            output,
            save_path
                .map(|p| p.join(format!("{}.png", component.handle.0)))
                .as_deref(),
        )? {
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
            } else {
                writeln!(w, "{:#}", output)?;
            }
        }

        writeln!(w)?;
    }

    Ok(())
}
