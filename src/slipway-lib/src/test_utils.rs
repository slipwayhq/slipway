use crate::errors::ComponentLoadError;
use crate::load::ComponentsLoader;
use crate::load::LoadedComponent;
use crate::utils::ch;
use crate::App;
use crate::Component;
use crate::ComponentCache;
use crate::ComponentHandle;
use crate::ComponentRigging;
use crate::Name;
use crate::Publisher;
use crate::Rigging;
use crate::Schema;
use crate::SlipwayId;
use crate::SlipwayReference;
use semver::Version;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

pub const TEST_PUBLISHER: &str = "test_publisher";

pub fn quote(s: &str) -> String {
    format!(r#""{}""#, s)
}

impl App {
    pub fn for_test(rigging: Rigging) -> App {
        App {
            publisher: Publisher::from_str(TEST_PUBLISHER).unwrap(),
            name: Name::from_str("test_name").unwrap(),
            version: Version::from_str("0.1.0").unwrap(),
            description: None,
            constants: Some(json!({"test_constant": "test_constant_value"})),
            rigging,
        }
    }
}

impl ComponentRigging {
    pub fn for_test(name: &str, input: Option<Value>) -> (ComponentHandle, ComponentRigging) {
        (
            ch(name),
            ComponentRigging {
                component: SlipwayReference::for_test(name),
                input,
                permissions: None,
            },
        )
    }
}

impl<TSchema> Component<TSchema> {
    pub fn for_test(
        reference: &SlipwayReference,
        input: TSchema,
        output: TSchema,
    ) -> Component<TSchema> {
        match reference {
            SlipwayReference::Registry {
                publisher,
                name,
                version,
            } => Component {
                publisher: Publisher::from_str(publisher).unwrap(),
                name: Name::from_str(name).unwrap(),
                version: version.clone(),
                description: None,
                input,
                output,
            },
            _ => unimplemented!("Only registry references are currently supported in this method"),
        }
    }
}

impl SlipwayId {
    pub fn for_test(name: &str, version: Version) -> Self {
        SlipwayId {
            publisher: Publisher::from_str(TEST_PUBLISHER).unwrap(),
            name: Name::from_str(name).unwrap(),
            version,
        }
    }
}

impl SlipwayReference {
    pub fn for_test_with_version(id: &str, version: &str) -> Self {
        SlipwayReference::Registry {
            publisher: TEST_PUBLISHER.to_string(),
            name: id.to_string(),
            version: Version::parse(version).expect("Invalid version"),
        }
    }

    pub fn for_test(id: &str) -> Self {
        SlipwayReference::Registry {
            publisher: TEST_PUBLISHER.to_string(),
            name: id.to_string(),
            version: Version::parse("0.1.0").expect("Invalid version"),
        }
    }
}

pub fn schema_any() -> Schema {
    Schema::JsonTypeDef {
        schema: jtd::Schema::Empty {
            definitions: Default::default(),
            metadata: Default::default(),
        },
    }
}

pub fn schema_valid(schema_name: &str, json: serde_json::Value) -> Schema {
    crate::parse_schema(schema_name, json).expect("schema should be valid")
}

pub(crate) struct MockComponentsLoader {
    pub schemas: HashMap<SlipwayReference, (Schema, Schema)>,
}

impl MockComponentsLoader {
    pub fn new(schemas: HashMap<String, (Schema, Schema)>) -> Self {
        let schemas = schemas
            .into_iter()
            .map(|(key, value)| (SlipwayReference::for_test(&key), value))
            .collect();

        MockComponentsLoader { schemas }
    }
}

impl ComponentsLoader for MockComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>> {
        component_references
            .iter()
            .map(|&component_reference| {
                self.schemas
                    .get(component_reference)
                    .map(|(input_schema, output_schema)| {
                        LoadedComponent::new(
                            component_reference,
                            serde_json::to_string(&Component::<Schema>::for_test(
                                component_reference,
                                input_schema.clone(),
                                output_schema.clone(),
                            ))
                            .expect("schema should serialize"),
                            Vec::new(),
                        )
                    })
                    .ok_or(ComponentLoadError::DefinitionLoadFailed {
                        reference: component_references[0].clone(),
                        error: "Schema not found".to_string(),
                    })
            })
            .collect()
    }
}

pub(crate) struct PermissiveMockComponentsLoader {}

impl PermissiveMockComponentsLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsLoader for PermissiveMockComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>> {
        component_references
            .iter()
            .map(|&component_reference| {
                let component_definition =
                    Component::<Schema>::for_test(component_reference, schema_any(), schema_any());

                let definition_string =
                    serde_json::to_string(&component_definition).expect("schema should serialize");

                Ok(LoadedComponent::new(
                    component_reference,
                    definition_string,
                    Vec::new(),
                ))
            })
            .collect()
    }
}

impl ComponentCache {
    pub fn for_test_with_schemas(app: &App, schemas: HashMap<String, (Schema, Schema)>) -> Self {
        ComponentCache::primed(app, &MockComponentsLoader::new(schemas)).unwrap()
    }

    pub fn for_test_permissive(app: &App) -> Self {
        ComponentCache::primed(app, &PermissiveMockComponentsLoader::new()).unwrap()
    }
}
