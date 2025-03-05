use crate::errors::ComponentLoadError;
use crate::errors::ComponentLoadErrorInner;
use crate::load::ComponentsLoader;
use crate::load::LoadedComponent;
use crate::utils::ch;
use crate::BasicComponentCache;
use crate::Component;
use crate::ComponentFiles;
use crate::ComponentFilesLoader;
use crate::ComponentHandle;
use crate::ComponentRigging;
use crate::Description;
use crate::Name;
use crate::Permission;
use crate::Permissions;
use crate::Publisher;
use crate::Rig;
use crate::Rigging;
use crate::Schema;
use crate::SlipwayId;
use crate::SlipwayReference;
use async_trait::async_trait;
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
            description: Some(Description::from_str("test_description").unwrap()),
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
            allow: Some(vec![Permission::All]),
            deny: None,
            callouts: None,
        }
    }

    pub fn for_test_with_reference_permissions(
        reference: SlipwayReference,
        input: Option<Value>,
        permissions: Permissions,
    ) -> ComponentRigging {
        ComponentRigging {
            component: reference,
            input,
            allow: Some(permissions.allow.clone()),
            deny: Some(permissions.deny.clone()),
            callouts: None,
        }
    }

    pub fn for_test_with_reference_callout_override(
        reference: SlipwayReference,
        input: Option<Value>,
        callout_handle: &str,
        callout_reference: SlipwayReference,
    ) -> ComponentRigging {
        ComponentRigging {
            component: reference,
            input,
            allow: Some(vec![Permission::All]),
            deny: None,
            callouts: Some(
                vec![(ch(callout_handle), callout_reference)]
                    .into_iter()
                    .collect(),
            ),
        }
    }

    pub fn for_test_with_reference_callout_override_permissions(
        reference: SlipwayReference,
        input: Option<Value>,
        callout_handle: &str,
        callout_reference: SlipwayReference,
        permissions: Permissions,
    ) -> ComponentRigging {
        ComponentRigging {
            component: reference,
            input,
            allow: Some(permissions.allow.clone()),
            deny: Some(permissions.deny.clone()),
            callouts: Some(
                vec![(ch(callout_handle), callout_reference)]
                    .into_iter()
                    .collect(),
            ),
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

pub async fn schema_valid(schema_name: &str, json: serde_json::Value) -> Schema {
    crate::load::parse_schema(
        schema_name,
        json,
        Arc::new(ComponentFiles::new(Box::new(MockSchemaResolver {}))),
    )
    .await
    .expect("schema should be valid")
}

pub struct MockSchemaResolver {}

#[async_trait]
impl ComponentFilesLoader for MockSchemaResolver {
    fn get_component_reference(&self) -> &SlipwayReference {
        unimplemented!();
    }

    fn get_component_path(&self) -> &std::path::Path {
        unimplemented!()
    }

    async fn exists(&self, _file_name: &str) -> Result<bool, ComponentLoadError> {
        unimplemented!()
    }

    async fn try_get_bin(
        &self,
        _file_name: &str,
    ) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        Ok(Some(Arc::new(serde_json::to_vec(&json!({})).unwrap())))
    }

    async fn try_get_text(
        &self,
        _file_name: &str,
    ) -> Result<Option<Arc<String>>, ComponentLoadError> {
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

fn no_component_files() -> Arc<ComponentFiles> {
    Arc::new(ComponentFiles::new(Box::new(NoComponentFiles {})))
}

#[async_trait]
impl ComponentFilesLoader for NoComponentFiles {
    fn get_component_reference(&self) -> &SlipwayReference {
        unimplemented!();
    }

    fn get_component_path(&self) -> &std::path::Path {
        unimplemented!()
    }

    async fn exists(&self, _file_name: &str) -> Result<bool, ComponentLoadError> {
        unimplemented!()
    }

    async fn try_get_bin(
        &self,
        _file_name: &str,
    ) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        unimplemented!()
    }

    async fn try_get_text(
        &self,
        _file_name: &str,
    ) -> Result<Option<Arc<String>>, ComponentLoadError> {
        unimplemented!()
    }
}

#[async_trait(?Send)]
impl ComponentsLoader for MockComponentsLoader {
    async fn load_components(
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
                            no_component_files(),
                        )
                    })
                    .ok_or(ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: format!("{}", component_reference),
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

#[async_trait(?Send)]
impl ComponentsLoader for PermissiveMockComponentsLoader {
    async fn load_components(
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
                    no_component_files(),
                ))
            })
            .collect()
    }
}

impl BasicComponentCache {
    pub async fn for_test_with_schemas(
        rig: &Rig,
        schemas: HashMap<String, (Schema, Schema)>,
    ) -> BasicComponentCache {
        BasicComponentCache::primed(rig, &MockComponentsLoader::new(schemas))
            .await
            .unwrap()
    }

    pub async fn for_test_permissive(rig: &Rig) -> BasicComponentCache {
        BasicComponentCache::primed(rig, &PermissiveMockComponentsLoader::new())
            .await
            .unwrap()
    }
}
