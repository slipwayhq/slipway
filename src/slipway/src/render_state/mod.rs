mod write_rig_graph;

use std::{io::Write, path::PathBuf};

use slipway_engine::RigExecutionState;

use crate::{
    canvas::render_canvas_if_exists,
    host_error::HostError,
    to_view_model::{to_view_model, ComponentViewModel},
};

pub(super) fn write_state<W: Write>(
    w: &mut W,
    state: &RigExecutionState<'_, '_>,
) -> Result<(), HostError> {
    write_state_with_outputs(w, None, state, PrintComponentOutputsType::None)
}

pub(super) fn write_state_with_outputs<W: Write>(
    w: &mut W,
    save_path: Option<&PathBuf>,
    state: &RigExecutionState<'_, '_>,
    outputs_type: PrintComponentOutputsType,
) -> Result<(), HostError> {
    let view_model = to_view_model(state);
    write_rig_graph::write_rig_graph(w, &view_model)?;
    writeln!(w)?;

    match outputs_type {
        PrintComponentOutputsType::None => {}
        PrintComponentOutputsType::LeafComponents => {
            for group in view_model.groups.iter() {
                for component in group.components.iter() {
                    if !component.output_row_indexes.is_empty() {
                        continue;
                    }

                    write_component_output(w, save_path, component)?;
                }
            }
        }
        PrintComponentOutputsType::AllComponents => {
            for group in view_model.groups.iter() {
                for component in group.components.iter() {
                    write_component_output(w, save_path, component)?;
                }
            }
        }
    }

    Ok(())
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

#[derive(Debug, Clone, Copy)]
pub(super) enum PrintComponentOutputsType {
    None,
    LeafComponents,
    AllComponents,
}
