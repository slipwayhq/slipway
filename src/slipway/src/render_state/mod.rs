mod write_rig_graph;

use std::io::Write;

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
    write_state_with_outputs(w, state, PrintComponentOutputsType::None)
}

pub(super) fn write_state_with_outputs<W: Write>(
    w: &mut W,
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

                    write_component_output(w, component)?;
                }
            }
        }
        PrintComponentOutputsType::AllComponents => {
            for group in view_model.groups.iter() {
                for component in group.components.iter() {
                    write_component_output(w, component)?;
                }
            }
        }
    }

    Ok(())
}

fn write_component_output<W: Write>(
    w: &mut W,
    component: &ComponentViewModel,
) -> Result<(), HostError> {
    if let Some(output) = component.state.output() {
        writeln!(w, r#"Component "{}" output:"#, component.handle)?;

        if !render_canvas_if_exists(component.handle, output, None)? {
            writeln!(w, "{}", output)?;
        }

        writeln!(w)?;
    }

    Ok(())
}

pub(super) enum PrintComponentOutputsType {
    None,
    LeafComponents,
    AllComponents,
}
