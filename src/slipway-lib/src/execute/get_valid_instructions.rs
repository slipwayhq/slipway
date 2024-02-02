use std::collections::{HashMap, HashSet};

use crate::types::primitives::ComponentHandle;

use super::{ComponentInput, ComponentOutput, ValidInstruction};

pub(crate) fn get_valid_instructions(
    inputs: &HashMap<ComponentHandle, ComponentInput>,
    outputs: &HashMap<ComponentHandle, ComponentOutput>,
    dependencies: &HashMap<ComponentHandle, HashSet<ComponentHandle>>,
) -> Vec<ValidInstruction> {
    let mut valid_instructions = HashSet::new();

    for (handle, dependencies) in dependencies.iter() {
        // We can always manually set the input for a component.
        valid_instructions.insert(ValidInstruction::SetInput {
            handle: handle.clone(),
        });

        // If there is any kind of input set for this component then we can get it, and also execute the component.
        if inputs.contains_key(handle) {
            valid_instructions.insert(ValidInstruction::GetInput {
                handle: handle.clone(),
            });

            valid_instructions.insert(ValidInstruction::ExecuteComponent {
                handle: handle.clone(),
            });
        }

        // If all dependencies of this component have their outputs then we evaluate the input for this component
        // and also execute the component.
        if dependencies.iter().all(|d| outputs.contains_key(d)) {
            valid_instructions.insert(ValidInstruction::EvaluateInput {
                handle: handle.clone(),
            });

            valid_instructions.insert(ValidInstruction::ExecuteComponent {
                handle: handle.clone(),
            });
        }

        // We can always manually set the output for a component.
        valid_instructions.insert(ValidInstruction::SetOutput {
            handle: handle.clone(),
        });

        // If this component has an output then we can get the output for this component.
        if outputs.contains_key(handle) {
            valid_instructions.insert(ValidInstruction::GetOutput {
                handle: handle.clone(),
            });
        }
    }

    if dependencies.keys().all(|h| outputs.contains_key(h)) {
        valid_instructions.insert(ValidInstruction::GetAppOutputs);
    }

    let mut result: Vec<ValidInstruction> = valid_instructions.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    fn create_test_components() -> HashMap<ComponentHandle, HashSet<ComponentHandle>> {
        // Dependency graph:
        // C
        // |\
        // B |
        // \ /
        //  A
        vec![
            (
                ComponentHandle::for_test("A"),
                vec![
                    ComponentHandle::for_test("B"),
                    ComponentHandle::for_test("C"),
                ],
            ),
            (
                ComponentHandle::for_test("B"),
                vec![ComponentHandle::for_test("C")],
            ),
            (ComponentHandle::for_test("C"), Vec::new()),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
    }

    #[test]
    fn when_no_inputs_or_outputs_it_should_return_valid_instructions() {
        let inputs = HashMap::new();
        let outputs = HashMap::new();
        let components_with_dependencies = create_test_components();

        let valid_instructions =
            get_valid_instructions(&inputs, &outputs, &components_with_dependencies);

        assert_eq!(
            valid_instructions,
            vec![
                ValidInstruction::SetInput {
                    handle: ComponentHandle::for_test("A")
                },
                ValidInstruction::SetInput {
                    handle: ComponentHandle::for_test("B")
                },
                ValidInstruction::SetInput {
                    handle: ComponentHandle::for_test("C")
                },
                ValidInstruction::EvaluateInput {
                    handle: ComponentHandle::for_test("C")
                },
                ValidInstruction::ExecuteComponent {
                    handle: ComponentHandle::for_test("C")
                },
                ValidInstruction::SetOutput {
                    handle: ComponentHandle::for_test("A")
                },
                ValidInstruction::SetOutput {
                    handle: ComponentHandle::for_test("B")
                },
                ValidInstruction::SetOutput {
                    handle: ComponentHandle::for_test("C")
                },
            ]
            .iter()
            .sorted()
            .cloned()
            .collect::<Vec<ValidInstruction>>()
        );
    }

    #[test]
    fn add_more_tests() {
        todo!();
    }
}
