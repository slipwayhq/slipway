mod host;
mod run_component_wasm;

use async_trait::async_trait;
pub use run_component_wasm::run_component_wasm;
use slipway_engine::{
    ComponentExecutionData, ComponentRunner, RunComponentError, TryRunComponentResult,
};
use slipway_host::SLIPWAY_COMPONENT_WASM_FILE_NAME;

pub const WASMTIME_COMPONENT_RUNNER_IDENTIFIER: &str = "wasmtime";

pub struct WasmComponentRunner {}

#[async_trait(?Send)]
impl ComponentRunner for WasmComponentRunner {
    fn identifier(&self) -> String {
        WASMTIME_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn run<'call>(
        &self,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let maybe_wasm_bytes = execution_data
            .context
            .files
            .try_get_bin(SLIPWAY_COMPONENT_WASM_FILE_NAME)
            .await?;

        let Some(wasm_bytes) = maybe_wasm_bytes else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_wasm(input, wasm_bytes, &execution_data.context).await?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}
