use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::{
    ComponentExecutionContext, ComponentRunner, RunComponentError, RunComponentResult, RunMetadata,
    SlipwayReference, SpecialComponentReference, TryRunComponentResult,
};

pub const SPECIAL_COMPONENT_RUNNER_IDENTIFIER: &str = "special";

pub struct SpecialComponentRunner {}

#[async_trait(?Send)]
impl ComponentRunner for SpecialComponentRunner {
    fn identifier(&self) -> String {
        SPECIAL_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    async fn run<'call>(
        &self,
        input: &serde_json::Value,
        context: &'call ComponentExecutionContext<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let reference = context.component_reference;

        let SlipwayReference::Special(inner) = reference else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        match inner {
            SpecialComponentReference::Passthrough => {
                let call_start = Instant::now();
                let output = input.clone();
                let call_duration = call_start.elapsed();

                Ok(TryRunComponentResult::Ran {
                    result: RunComponentResult {
                        output,
                        metadata: RunMetadata {
                            prepare_input_duration: Duration::ZERO,
                            prepare_component_duration: Duration::ZERO,
                            call_duration,
                            process_output_duration: Duration::ZERO,
                        },
                    },
                })
            }
            SpecialComponentReference::Sink => Ok(TryRunComponentResult::Ran {
                result: RunComponentResult {
                    output: serde_json::json!({}),
                    metadata: RunMetadata {
                        prepare_input_duration: Duration::ZERO,
                        prepare_component_duration: Duration::ZERO,
                        call_duration: Duration::ZERO,
                        process_output_duration: Duration::ZERO,
                    },
                },
            }),
        }
    }
}
