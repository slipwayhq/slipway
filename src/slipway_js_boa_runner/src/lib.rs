use async_trait::async_trait;
use serde::Deserialize;
use slipway_engine::{
    ComponentExecutionContext, ComponentRunner, RunComponentError, TryRunComponentResult,
};

mod async_environment;
mod boa_environment;
mod component_module_loader;
mod host;
mod run_component_javascript;

const BOA_COMPONENT_RUNNER_IDENTIFIER: &str = "js_boa";
const BOA_COMPONENT_DEFINITION_FILE_NAME: &str = "js_component.json";
const BOA_RUN_JS_FILE_NAME: &str = "run.js";

pub struct BoaComponentRunner {}

#[async_trait(?Send)]
impl ComponentRunner for BoaComponentRunner {
    fn identifier(&self) -> String {
        BOA_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn run<'call>(
        &self,
        input: &serde_json::Value,
        context: &'call ComponentExecutionContext<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let maybe_run_js = context.files.try_get_text(BOA_RUN_JS_FILE_NAME).await?;

        let Some(run_js) = maybe_run_js else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let maybe_boa_definition = context
            .files
            .try_get_json::<BoaComponentDefinition>(BOA_COMPONENT_DEFINITION_FILE_NAME)
            .await?;

        let run_result = run_component_javascript::run_component_javascript(
            input,
            run_js,
            maybe_boa_definition,
            context,
        )
        .await?;

        Ok(TryRunComponentResult::Ran { result: run_result })
    }
}

#[derive(Debug, Deserialize)]
struct BoaComponentDefinition {
    scripts: Vec<String>,
}
