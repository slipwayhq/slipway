use serde::Deserialize;
use slipway_engine::{
    ComponentExecutionData, ComponentRunner, RunComponentError, TryRunComponentResult,
};

mod run_component_javascript;

const BOA_COMPONENT_RUNNER_IDENTIFIER: &str = "js_boa";
const BOA_COMPONENT_DEFINITION_FILE_NAME: &str = "slipway_js_component.json";

pub struct BoaComponentRunner {}

impl ComponentRunner for BoaComponentRunner {
    fn identifier(&self) -> String {
        BOA_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    fn run<'call>(
        &self,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let maybe_boa_definition = execution_data
            .context
            .files
            .try_get_json::<BoaComponentDefinition>(BOA_COMPONENT_DEFINITION_FILE_NAME)?;

        let Some(boa_definition) = maybe_boa_definition else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        let run_result = run_component_javascript::run_component_javascript(
            input,
            boa_definition,
            &execution_data.context,
        )?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

#[derive(Debug, Deserialize)]
struct BoaComponentDefinition {
    scripts: Vec<String>,
}
