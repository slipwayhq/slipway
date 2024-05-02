use crate::errors::ComponentLoadError;
use crate::execute::app_session::AppSessionOptions;
use crate::execute::load_components::ComponentPartLoader;
use crate::execute::load_components::InMemoryComponentCache;
use crate::execute::load_components::LoaderId;
use crate::utils::ch;
use crate::App;
use crate::AppSession;
use crate::Component;
use crate::ComponentHandle;
use crate::ComponentRigging;
use crate::Name;
use crate::Publisher;
use crate::Rigging;
use crate::SlipwayId;
use crate::SlipwayReference;
use async_trait::async_trait;
use semver::Version;
use serde_json::json;
use serde_json::Value;
use std::cell::RefCell;
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

impl AppSession {
    pub fn for_test_with_schemas(
        app: App,
        schemas: HashMap<String, (jtd::Schema, jtd::Schema)>,
    ) -> Self {
        // Map keys to use SlipwayReference::for_test_from_id(key)
        let schemas = schemas
            .into_iter()
            .map(|(key, value)| (SlipwayReference::for_test(&key), value))
            .collect();

        let options = AppSessionOptions::default();
        AppSession {
            app,
            component_cache: RefCell::new(Box::new(InMemoryComponentCache::new(
                vec![Box::new(MockComponentLoader { schemas })],
                vec![Box::new(MockComponentLoader {
                    schemas: HashMap::new(),
                })],
            ))),
            component_load_error_behavior: options.component_load_error_behavior,
        }
    }

    pub fn for_test(app: App) -> Self {
        let options = AppSessionOptions::default();
        AppSession {
            app,
            component_cache: RefCell::new(Box::new(InMemoryComponentCache::new(
                vec![Box::new(LooseMockComponentLoader {})],
                vec![Box::new(LooseMockComponentLoader {})],
            ))),
            component_load_error_behavior: options.component_load_error_behavior,
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

pub fn schema_any() -> jtd::Schema {
    jtd::Schema::Empty {
        definitions: Default::default(),
        metadata: Default::default(),
    }
}

pub(crate) struct MockComponentLoader {
    pub schemas: HashMap<SlipwayReference, (jtd::Schema, jtd::Schema)>,
}

#[async_trait]
impl ComponentPartLoader<Component<jtd::Schema>> for MockComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("Mock").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<Component<jtd::Schema>>, ComponentLoadError> {
        self.schemas
            .get(component_reference)
            .map(|(input_schema, output_schema)| {
                Component::<jtd::Schema>::for_test(
                    component_reference,
                    input_schema.clone(),
                    output_schema.clone(),
                )
            })
            .map_or(Ok(None), |component| Ok(Some(component)))
    }
}

#[async_trait]
impl ComponentPartLoader<Vec<u8>> for MockComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("Mock").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        _component_reference: &SlipwayReference,
    ) -> Result<Option<Vec<u8>>, ComponentLoadError> {
        unimplemented!();
    }
}

pub(crate) struct LooseMockComponentLoader {}

#[async_trait]
impl ComponentPartLoader<Component<jtd::Schema>> for LooseMockComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("Mock").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Option<Component<jtd::Schema>>, ComponentLoadError> {
        Ok(Some(Component::<jtd::Schema>::for_test(
            component_reference,
            schema_any(),
            schema_any(),
        )))
    }
}

#[async_trait]
impl ComponentPartLoader<Vec<u8>> for LooseMockComponentLoader {
    fn id(&self) -> LoaderId {
        LoaderId::from_str("Mock").expect("LoaderId should be valid")
    }

    async fn load(
        &self,
        _component_reference: &SlipwayReference,
    ) -> Result<Option<Vec<u8>>, ComponentLoadError> {
        unimplemented!();
    }
}
