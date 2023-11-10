mod component_reference;
mod resolved_component_reference;
mod unresolved_component_reference;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(test)]
pub(crate) const TEST_PUBLISHER: &str = "test-publisher";

pub use self::{
    resolved_component_reference::ResolvedComponentReference,
    unresolved_component_reference::UnresolvedComponentReference,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub publisher: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Version,
    pub inputs: Vec<ComponentInput>,
    pub output: ComponentOutput,
}

impl Component {
    pub fn get_reference(&self) -> ResolvedComponentReference {
        ResolvedComponentReference::new(&self.publisher, &self.name, &self.version)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentOutput {
    pub schema: Option<Value>,
    pub schema_reference: Option<UnresolvedComponentReference>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentInput {
    pub id: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Value>, // Either specify the schema for the default_value, or override the schema in the default_component.
    pub default_component: Option<ComponentInputSpecification>,
    pub default_value: Option<Value>,
}

impl ComponentInput {
    pub fn get_display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentInputSpecification {
    pub reference: UnresolvedComponentReference,
    pub input_overrides: Option<Vec<ComponentInputOverride>>, // Override the input defaults.
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentInputOverride {
    pub id: String,
    pub component: Option<ComponentInputSpecification>, // Override the component defaults.
    pub value: Option<Value>,                           // Set an explicit value for this input.
}
