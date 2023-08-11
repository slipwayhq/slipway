mod component_reference;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) use self::component_reference::ComponentReference;

#[derive(Serialize, Deserialize)]
pub(crate) struct Component {
    pub id: String,
    pub description: Option<String>,
    pub version: String,
    pub inputs: Vec<ComponentInput>,
    pub output: ComponentOutput,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentOutput {
    pub schema: Option<Value>,
    pub schema_reference: Option<ComponentReference>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Value>,
    pub default_component: Option<ComponentInputSpecification>,
    pub default_value: Option<Value>,
}

impl ComponentInput {
    pub fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentInputSpecification {
    pub reference: ComponentReference,
    pub inputs: Option<Vec<ComponentInputOverride>>, // Override the input defaults.
    pub output: Option<Value>,                       // Set the output value explicitly.
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ComponentInputOverride {
    pub id: String,
    pub component: Option<ComponentInputSpecification>,
    pub value: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_deserialize_complex_component_rigging() {
        let json = r#"
        {
            "id": "test",
            "description": "Test component",
            "version": "1.0.0",
            "inputs": [
                {
                    "id": "input1",
                    "name": "Input 1",
                    "description": "The first input",
                    "schema": {
                        "type": "string"
                    },
                    "default_component": {
                        "reference": {
                            "id": "test2",
                            "version": "1.0.0"
                        },
                        "inputs": [
                            {
                                "id": "input1",
                                "value": "test"
                            }
                        ]
                    }
                },
                {
                    "id": "input2",
                    "name": "Input 2",
                    "description": "The second input",
                    "schema": {
                        "type": "string"
                    },
                    "default_value": "test"
                }
            ],
            "output": {
                "schema": {
                    "type": "string"
                },
                "schema_reference": {
                    "id": "test2",
                    "version": "1.0.0"
                }
            }
        }"#;

        let _component: Component = serde_json::from_str(json).unwrap();
    }
}
