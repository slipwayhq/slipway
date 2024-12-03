use std::time::{Duration, Instant};

use crate::{
    ComponentExecutionData, ComponentHandle, ComponentRunner, RunComponentError,
    RunComponentResult, RunMetadata, SlipwayReference, SpecialComponentReference,
    TryRunComponentResult,
};

pub const SPECIAL_COMPONENT_RUNNER_IDENTIFIER: &str = "special";

pub struct SpecialComponentRunner {}

impl ComponentRunner for SpecialComponentRunner {
    fn identifier(&self) -> String {
        SPECIAL_COMPONENT_RUNNER_IDENTIFIER.to_string()
    }

    fn run<'call>(
        &self,
        handle: &'call ComponentHandle,
        execution_data: &'call ComponentExecutionData<'call, '_, '_>,
    ) -> Result<TryRunComponentResult, RunComponentError> {
        let reference = execution_data.get_component_reference(handle);

        let SlipwayReference::Special(inner) = reference else {
            return Ok(TryRunComponentResult::CannotRun);
        };

        let input = &execution_data.input.value;

        match inner {
            SpecialComponentReference::Pass => {
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
