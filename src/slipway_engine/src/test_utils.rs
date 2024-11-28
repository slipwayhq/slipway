use crate::errors::ComponentLoadError;
use crate::errors::ComponentLoadErrorInner;
use crate::load::ComponentsLoader;
use crate::load::LoadedComponent;
use crate::utils::ch;
use crate::Component;
use crate::ComponentCache;
use crate::ComponentFiles;
use crate::ComponentHandle;
use crate::ComponentRigging;
use crate::Name;
use crate::Publisher;
use crate::Rig;
use crate::Rigging;
use crate::Schema;
use crate::SlipwayId;
use crate::SlipwayReference;
use semver::Version;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

pub const TEST_PUBLISHER: &str = "test_publisher";

impl Rig {
    pub fn for_test(rigging: Rigging) -> Rig {
        Rig {
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
            ComponentRigging::for_test_with_reference(SlipwayReference::for_test(name), input),
        )
    }

    pub fn for_test_with_reference(
        reference: SlipwayReference,
        input: Option<Value>,
    ) -> ComponentRigging {
        ComponentRigging {
            component: reference,
            input,
            permissions: None,
            callouts: None,
        }
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
                constants: None,
                rigging: None,
                callouts: None,
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
            version: Version::parse(version).expect("Version should be valid"),
        }
    }

    pub fn for_test(id: &str) -> Self {
        SlipwayReference::Registry {
            publisher: TEST_PUBLISHER.to_string(),
            name: id.to_string(),
            version: Version::parse("0.1.0").expect("Version should be valid"),
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
    crate::parse::parse_schema(schema_name, json, Arc::new(MockSchemaResolver {}))
        .expect("schema should be valid")
}

pub struct MockSchemaResolver {}

impl ComponentFiles for MockSchemaResolver {
    fn get_json(&self, _file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        Ok(Arc::new(json!({})))
    }

    fn get_component_reference(&self) -> &SlipwayReference {
        unimplemented!();
    }

    fn get_component_path(&self) -> &std::path::Path {
        unimplemented!()
    }

    fn exists(&self, _file_name: &str) -> Result<bool, ComponentLoadError> {
        unimplemented!()
    }

    fn try_get_bin(&self, _file_name: &str) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        unimplemented!()
    }

    fn try_get_text(&self, _file_name: &str) -> Result<Option<Arc<String>>, ComponentLoadError> {
        unimplemented!()
    }
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

struct NoComponentFiles {}

impl ComponentFiles for NoComponentFiles {
    fn get_component_reference(&self) -> &SlipwayReference {
        unimplemented!();
    }

    fn get_component_path(&self) -> &std::path::Path {
        unimplemented!()
    }

    fn exists(&self, _file_name: &str) -> Result<bool, ComponentLoadError> {
        unimplemented!()
    }

    fn try_get_bin(&self, _file_name: &str) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        unimplemented!()
    }

    fn try_get_text(&self, _file_name: &str) -> Result<Option<Arc<String>>, ComponentLoadError> {
        unimplemented!()
    }
}

impl ComponentsLoader for MockComponentsLoader {
    fn load_components(
        &self,
        component_references: &[SlipwayReference],
    ) -> Vec<Result<LoadedComponent, ComponentLoadError>> {
        component_references
            .iter()
            .map(|component_reference| {
                self.schemas
                    .get(component_reference)
                    .map(|(input_schema, output_schema)| {
                        LoadedComponent::new(
                            component_reference.clone(),
                            serde_json::to_string(&Component::<Schema>::for_test(
                                component_reference,
                                input_schema.clone(),
                                output_schema.clone(),
                            ))
                            .expect("schema should serialize"),
                            Arc::new(NoComponentFiles {}),
                        )
                    })
                    .ok_or(ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: format!("{:?}", component_reference),
                            error: "Component not found in map".to_string(),
                        },
                    ))
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
    fn load_components(
        &self,
        component_references: &[SlipwayReference],
    ) -> Vec<Result<LoadedComponent, ComponentLoadError>> {
        component_references
            .iter()
            .map(|component_reference| {
                let component_definition =
                    Component::<Schema>::for_test(component_reference, schema_any(), schema_any());

                let definition_string =
                    serde_json::to_string(&component_definition).expect("schema should serialize");

                Ok(LoadedComponent::new(
                    component_reference.clone(),
                    definition_string,
                    Arc::new(NoComponentFiles {}),
                ))
            })
            .collect()
    }
}

impl ComponentCache {
    pub fn for_test_with_schemas(rig: &Rig, schemas: HashMap<String, (Schema, Schema)>) -> Self {
        ComponentCache::primed(rig, &MockComponentsLoader::new(schemas)).unwrap()
    }

    pub fn for_test_permissive(rig: &Rig) -> Self {
        ComponentCache::primed(rig, &PermissiveMockComponentsLoader::new()).unwrap()
    }
}
