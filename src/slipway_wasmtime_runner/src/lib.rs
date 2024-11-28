mod run_component_wasm;

pub use run_component_wasm::run_component_wasm;
use slipway_host::{
    run::{errors::RunComponentError, ComponentRunner},
    RunComponentResult, SLIPWAY_COMPONENT_WASM_FILE_NAME,
};
use slipway_engine::{errors::ComponentLoadError, ComponentExecutionData, ComponentHandle};

pub const WASMTIME_COMPONENT_RUNNER_IDENTIFIER: &str = "Wasmtime";
pub struct WasmComponentRunner {}

impl<'rig> ComponentRunner<'rig> for WasmComponentRunner {
    fn identifier(&self) -> String {
        WASMTIME_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    fn can_run_component(
        &self,
        _handle: &ComponentHandle,
        execution_data: ComponentExecutionData<'rig>,
    ) -> Result<bool, ComponentLoadError> {
        execution_data
            .files
            .exists(SLIPWAY_COMPONENT_WASM_FILE_NAME)
    }

    fn run_component(
        &self,
        handle: &ComponentHandle,
        execution_data: ComponentExecutionData<'rig>,
    ) -> Result<RunComponentResult, RunComponentError> {
        let wasm_bytes = execution_data
            .files
            .get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME)?;

        let input = &execution_data.input.value;

        run_component_wasm(handle, input, wasm_bytes)
    }
}
