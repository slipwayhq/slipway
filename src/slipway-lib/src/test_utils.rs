use crate::errors::ComponentLoadError;
use crate::errors::ComponentLoadErrorInner;
use crate::load::ComponentsLoader;
use crate::load::LoadedComponent;
use crate::utils::ch;
use crate::Rig;
use crate::Component;
use crate::ComponentCache;
use crate::ComponentHandle;
use crate::ComponentJson;
use crate::ComponentRigging;
use crate::ComponentWasm;
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
use std::sync::Arc;

pub const TEST_PUBLISHER: &str = "test_publisher";

pub fn quote(s: &str) -> String {
    format!(r#""{}""#, s)
}

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
    crate::parse::parse_schema(schema_name, json, Arc::new(MockSchemaResolver {}))
        .expect("schema should be valid")
}

pub struct MockSchemaResolver {}

impl ComponentJson for MockSchemaResolver {
    fn get(&self, _file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        Ok(Arc::new(json!({})))
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

struct NoComponentWasm {}

impl ComponentWasm for NoComponentWasm {
    fn get(&self) -> Result<Arc<Vec<u8>>, ComponentLoadError> {
        panic!("NoComponentWasm should not be executed");
    }
}

struct NoComponentJson {}

impl ComponentJson for NoComponentJson {
    fn get(&self, _file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        panic!("NoComponentJson should not be executed");
    }
}

impl ComponentsLoader for MockComponentsLoader {
    fn load_components<'rig>(
        &self,
        component_references: &[&'rig SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'rig>, ComponentLoadError>> {
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
                            Arc::new(NoComponentWasm {}),
                            Arc::new(NoComponentJson {}),
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
    fn load_components<'rig>(
        &self,
        component_references: &[&'rig SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'rig>, ComponentLoadError>> {
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
                    Arc::new(NoComponentWasm {}),
                    Arc::new(NoComponentJson {}),
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

#[cfg(test)]
pub(crate) mod http_server {
    use std::collections::HashMap;
    use std::sync::mpsc;
    use std::sync::mpsc::Sender;
    use std::thread;
    use tiny_http::Response;
    use tiny_http::Server;

    // Simple test server used for testing things like schema HTTP resolution
    // is working as expected.
    pub(crate) struct TestServer {
        stop_signal: Sender<char>,
        server_thread: thread::JoinHandle<()>,
    }

    impl TestServer {
        pub fn start(responses: HashMap<String, String>) -> Self {
            let (tx, rx) = mpsc::channel();

            let server = Server::http("0.0.0.0:8080").unwrap();
            let server_thread = thread::spawn(move || loop {
                // Check for stop signal in a non-blocking way
                if rx.try_recv().is_ok() {
                    break;
                }

                // Handle incoming requests
                if let Ok(Some(request)) =
                    server.recv_timeout(std::time::Duration::from_millis(100))
                {
                    match responses.get(request.url()) {
                        None => {
                            println!("Not found: {}", request.url());
                            request
                                .respond(Response::from_string("Not found").with_status_code(404))
                                .unwrap();
                            continue;
                        }
                        Some(response_str) => {
                            request
                                .respond(Response::from_string(response_str))
                                .unwrap();
                        }
                    }
                }
            });

            TestServer {
                stop_signal: tx,
                server_thread,
            }
        }

        pub fn stop(self) {
            self.stop_signal.send('a').unwrap();
            self.server_thread.join().unwrap();
        }
    }
}
