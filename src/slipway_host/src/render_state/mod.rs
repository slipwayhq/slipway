pub mod to_view_model;
mod write_rig_graph;

use std::{io::Write, path::PathBuf};

use slipway_engine::RigExecutionState;

use crate::render_state::to_view_model::{RigExecutionStateViewModel, to_view_model};

pub fn write_state<'state, W: Write, TError: From<std::io::Error>>(
    w: &mut W,
    state: &'state RigExecutionState<'_, '_>,
) -> Result<RigExecutionStateViewModel<'state>, TError> {
    let view_model = to_view_model(state);
    writeln!(w)?;
    write_rig_graph::write_rig_graph(w, &view_model)?;
    writeln!(w)?;
    Ok(view_model)
}

pub trait WriteComponentOutputs<W: Write, TError> {
    fn write_component_outputs(
        &self,
        w: &mut W,
        save_path: Option<&PathBuf>,
        view_model: &RigExecutionStateViewModel,
    ) -> Result<(), TError>;
}
