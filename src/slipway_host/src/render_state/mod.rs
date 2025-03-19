pub mod to_view_model;
mod write_rig_graph;

use std::{io::Write, path::PathBuf};

use slipway_engine::RigExecutionState;

use crate::render_state::to_view_model::{RigExecutionStateViewModel, to_view_model};

pub fn write_state<W: Write, TError: From<std::io::Error>>(
    w: &mut W,
    state: &RigExecutionState<'_, '_>,
) -> Result<(), TError> {
    write_state_with_outputs(w, None, state, SinkWriteComponentsOutputs::<TError>::new())
}

pub fn write_state_with_outputs<
    W: Write,
    P: WriteComponentOutputs<W, TError>,
    TError: From<std::io::Error>,
>(
    w: &mut W,
    save_path: Option<&PathBuf>,
    state: &RigExecutionState<'_, '_>,
    write_component_outputs: P,
) -> Result<(), TError> {
    let view_model = to_view_model(state);
    writeln!(w)?;
    write_rig_graph::write_rig_graph(w, &view_model)?;
    writeln!(w)?;

    write_component_outputs.write_component_outputs(w, save_path, view_model)?;

    Ok(())
}

pub trait WriteComponentOutputs<W: Write, TError> {
    fn write_component_outputs(
        &self,
        w: &mut W,
        save_path: Option<&PathBuf>,
        view_model: RigExecutionStateViewModel,
    ) -> Result<(), TError>;
}

struct SinkWriteComponentsOutputs<TError: From<std::io::Error>> {
    _phantom: std::marker::PhantomData<TError>,
}

impl<TError: From<std::io::Error>> SinkWriteComponentsOutputs<TError> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<W: Write, TError: From<std::io::Error>> WriteComponentOutputs<W, TError>
    for SinkWriteComponentsOutputs<TError>
{
    fn write_component_outputs(
        &self,
        _w: &mut W,
        _save_path: Option<&PathBuf>,
        _view_model: RigExecutionStateViewModel,
    ) -> Result<(), TError> {
        Ok(())
    }
}
