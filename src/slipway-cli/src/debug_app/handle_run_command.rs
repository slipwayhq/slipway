use slipway_lib::{AppExecutionState, ComponentHandle, Immutable, Instruction};

use crate::run_component_wasm::run_component_wasm;

use super::errors::SlipwayDebugError;

pub(super) fn handle_run_command<'app>(
    handle: &'app ComponentHandle,
    state: &AppExecutionState<'app>,
) -> Result<Immutable<AppExecutionState<'app>>, SlipwayDebugError> {
    let execution_data = state.get_component_execution_data(handle)?;

    let output = run_component_wasm(execution_data)?;

    let new_state = state.step(Instruction::SetOutput {
        handle: handle.clone(),
        value: output,
    })?;

    Ok(new_state)
}
