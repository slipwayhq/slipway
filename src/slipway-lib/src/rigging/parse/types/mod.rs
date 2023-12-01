mod resolved_component_reference;
mod unresolved_component_reference;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

#[cfg(test)]
impl Component {
    pub fn for_test(
        name: &str,
        version: Version,
        inputs: Vec<ComponentInput>,
        output: ComponentOutput,
    ) -> Self {
        Self {
            name: name.to_string(),
            publisher: TEST_PUBLISHER.to_string(),
            description: None,
            version,
            inputs,
            output,
        }
    }
}

#[cfg(test)]
impl ComponentInput {
    pub fn for_test(
        id: &str,
        default_component: Option<ComponentInputSpecification>,
        default_value: Option<Value>,
    ) -> Self {
        Self {
            id: id.to_string(),
            display_name: None,
            description: None,
            schema: None,
            default_component,
            default_value,
        }
    }
    pub fn for_test_with_display_name(id: &str, display_name: &str) -> Self {
        Self {
            id: id.to_string(),
            display_name: Some(display_name.to_string()),
            description: None,
            schema: None,
            default_component: None,
            default_value: Some(json!(1)),
        }
    }
}

#[cfg(test)]
impl ComponentOutput {
    pub fn for_test(
        schema: Option<Value>,
        schema_reference: Option<UnresolvedComponentReference>,
    ) -> Self {
        Self {
            schema,
            schema_reference,
        }
    }

    pub fn for_test_with_schema() -> Self {
        Self {
            schema: Some(json!({
                "schema": {
                    "type": "string"
                }
            })),
            schema_reference: None,
        }
    }
}

#[cfg(test)]
impl ComponentInputSpecification {
    pub fn for_test(
        reference: UnresolvedComponentReference,
        input_overrides: Option<Vec<ComponentInputOverride>>,
    ) -> Self {
        Self {
            reference,
            input_overrides,
        }
    }
}

#[cfg(test)]
impl ComponentInputOverride {
    pub fn for_test_with_component(id: &str, component: ComponentInputSpecification) -> Self {
        Self {
            id: id.to_string(),
            component: Some(component),
            value: None,
        }
    }
}
