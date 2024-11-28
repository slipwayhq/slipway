mod run_component_wasm;

pub use run_component_wasm::run_component_wasm;
use slipway_engine::{ComponentExecutionData, ComponentHandle};
use slipway_host::{
    run::{errors::RunComponentError, ComponentRunner, ComponentRunnerResult},
    SLIPWAY_COMPONENT_WASM_FILE_NAME,
};

pub const WASMTIME_COMPONENT_RUNNER_IDENTIFIER: &str = "Wasmtime";

pub struct WasmComponentRunner {}

impl<'rig> ComponentRunner<'rig> for WasmComponentRunner {
    fn identifier(&self) -> String {
        WASMTIME_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    fn run(
        &self,
        handle: &ComponentHandle,
        execution_data: ComponentExecutionData<'rig>,
    ) -> Result<ComponentRunnerResult, RunComponentError> {
        let maybe_wasm_bytes = execution_data
            .files
            .try_get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME)?;

        let Some(wasm_bytes) = maybe_wasm_bytes else {
            return Ok(ComponentRunnerResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_wasm(handle, input, wasm_bytes)?;

        Ok(ComponentRunnerResult::Ran { result: run_result })
    }
}
