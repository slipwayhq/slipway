use async_trait::async_trait;
use serde::Deserialize;
use slipway_engine::{
    ComponentExecutionData, ComponentRunner, RunComponentError, TryRunComponentResult,
};

mod deno_ops;
mod run_component_javascript;

const DENO_COMPONENT_RUNNER_IDENTIFIER: &str = "js_deno";
const DENO_COMPONENT_DEFINITION_FILE_NAME: &str = "slipway_js_component.json";

pub struct DenoComponentRunner {}

#[async_trait(?Send)]
impl ComponentRunner for DenoComponentRunner {
    fn identifier(&self) -> String {
        DENO_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn run<'call>(
        &self,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let maybe_deno_definition = execution_data
            .context
            .files
            .try_get_json::<DenoComponentDefinition>(DENO_COMPONENT_DEFINITION_FILE_NAME)
            .await?;

        let Some(deno_definition) = maybe_deno_definition else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_javascript::run_component_javascript(
            input,
            deno_definition,
            &execution_data.context,
        )
        .await?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

#[derive(Debug, Deserialize)]
struct DenoComponentDefinition {
    scripts: Vec<String>,
}
