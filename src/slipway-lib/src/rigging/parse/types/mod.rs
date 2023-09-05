mod component_reference;
mod resolved_component_reference;
mod unresolved_component_reference;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use self::component_reference::ComponentReference;

#[derive(Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub description: Option<String>,
    pub version: String,
    pub inputs: Vec<ComponentInput>,
    pub output: ComponentOutput,
}

impl Component {
    pub fn get_reference(&self) -> ComponentReference {
        ComponentReference::exact(&self.id, &self.version)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ComponentOutput {
    pub schema: Option<Value>,
    pub schema_reference: Option<ComponentReference>,
}

#[derive(Serialize, Deserialize)]
pub struct ComponentInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Value>, // Either specify the schema for the default_value, or override the schema in the default_component.
    pub default_component: Option<ComponentInputSpecification>,
    pub default_value: Option<Value>,
}

impl ComponentInput {
    pub fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ComponentInputSpecification {
    pub reference: ComponentReference,
    pub input_overrides: Option<Vec<ComponentInputOverride>>, // Override the input defaults.
}

#[derive(Serialize, Deserialize)]
pub struct ComponentInputOverride {
    pub id: String,
    pub component: Option<ComponentInputSpecification>, // Override the component defaults.
    pub value: Option<Value>,                           // Set an explicit value for this input.
}
