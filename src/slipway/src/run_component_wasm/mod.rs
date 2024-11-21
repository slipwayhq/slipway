use slipway_lib::{ComponentExecutionData, ComponentHandle, RunMetadata};

use self::errors::WasmExecutionError;

mod run_as_wasm_component;

pub(super) mod errors;

pub(super) struct RunComponentWasmResult {
    pub output: serde_json::Value,
    pub metadata: RunMetadata,
}

pub(super) fn run_component_wasm(
    execution_data: ComponentExecutionData,
    handle: &ComponentHandle,
) -> Result<RunComponentWasmResult, WasmExecutionError> {
    run_as_wasm_component::run_component_wasm(execution_data, handle)
}
