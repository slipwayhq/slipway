use super::primitives::Hash;
use std::{collections::HashSet, rc::Rc};

use crate::{ComponentHandle, ComponentRigging, JsonMetadata};

#[derive(Clone, Debug)]
pub struct ComponentState<'app> {
    pub handle: &'app ComponentHandle,
    pub rigging: &'app ComponentRigging,
    pub dependencies: HashSet<&'app ComponentHandle>,
    pub input_override: Option<Rc<ComponentInputOverride>>,
    pub output_override: Option<Rc<ComponentOutputOverride>>,
    pub execution_input: Option<Rc<ComponentInput>>,
    pub execution_output: Option<Rc<ComponentOutput>>,
}

impl<'app> ComponentState<'app> {
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
    pub metadata: JsonMetadata,
}

#[derive(Debug)]
pub struct ComponentInputOverride {
    pub value: serde_json::Value,
}

#[derive(Debug)]
pub struct ComponentOutput {
    pub value: serde_json::Value,
    pub input_hash_used: Hash,
    pub metadata: JsonMetadata,
}

#[derive(Debug)]
pub struct ComponentOutputOverride {
    pub value: serde_json::Value,
    pub metadata: JsonMetadata,
}
