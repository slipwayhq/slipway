use super::primitives::Hash;
use std::{collections::HashSet, sync::Arc};

use crate::{ComponentHandle, ComponentRigging, JsonMetadata, RunMetadata};

// NOTE: The Arcs in this struct can actually be Rc, because we run in a single
// threaded async runtime. However, because Wasmtime requires Send we are using
// Arc to keep it happy.

#[derive(Clone, Debug)]
pub struct ComponentState<'rig> {
    pub handle: &'rig ComponentHandle,
    pub rigging: &'rig ComponentRigging,
    pub dependencies: HashSet<&'rig ComponentHandle>,

    /// The overridden input, with references unresolved. This overrides the input defined
    /// in the rigging.
    pub input_override: Option<Arc<ComponentInputOverride>>,

    /// The overridden output. This overrides the `execution_output`.
    pub output_override: Option<Arc<ComponentOutputOverride>>,

    /// When a component is ready to be executed, this will contain the input with all references resolved.
    pub execution_input: Option<Arc<ComponentInput>>,

    /// The output of the component after it has been executed.
    pub execution_output: Option<Arc<ComponentOutput>>,
}

impl ComponentState<'_> {
    /// Get the input of the component, which is either the input_override or
    /// the input or None.
    pub fn input(&self) -> Option<&serde_json::Value> {
        match self.input_override.as_ref() {
            Some(input_override) => Some(&input_override.value),
            None => self.rigging.input.as_ref(),
        }
    }

    /// Get the output of the component, which is either the output_override or
    /// the execution_output or None.
    pub fn output(&self) -> Option<&serde_json::Value> {
        match self.output_override.as_ref() {
            Some(output_override) => Some(&output_override.value),
            None => self.execution_output.as_ref().map(|output| &output.value),
        }
    }
}

#[derive(Debug)]
pub struct ComponentInput {
    pub value: serde_json::Value,
    pub json_metadata: JsonMetadata,
}

#[derive(Debug)]
pub struct ComponentInputOverride {
    pub value: serde_json::Value,
}

#[derive(Debug)]
pub struct ComponentOutput {
    pub value: serde_json::Value,
    pub input_hash_used: Hash,
    pub json_metadata: JsonMetadata,
    pub run_metadata: RunMetadata,
}

#[derive(Debug)]
pub struct ComponentOutputOverride {
    pub value: serde_json::Value,
    pub json_metadata: JsonMetadata,
}
