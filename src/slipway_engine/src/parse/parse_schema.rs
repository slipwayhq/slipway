use std::sync::Arc;

use anyhow::Context;
use jsonschema::{SchemaResolver, SchemaResolverError, Validator};
use url::Url;

use crate::{errors::ComponentLoadErrorInner, ComponentFiles};

use super::types::Schema;

const DEFAULT_BASE_URL_PREFIX: &str = "file:///";

pub fn parse_schema(
    schema_name: &str,
    schema: serde_json::Value,
    component_files: Arc<ComponentFiles>,
) -> Result<Schema, ComponentLoadErrorInner> {
    if let Some(serde_json::Value::String(schema_uri)) = schema.get("$schema") {
        if schema_uri.contains("://json-schema.org/") {
            // If the schema contains a $schema field that refers to a JSON Schema
            // then we parse it as a JSON Schema.
            return parse_json_schema(schema, schema_name, component_files);
        }
    }

    // Otherwise we default to JsonTypeDef.
    parse_json_typedef_schema(schema, schema_name)
}

fn parse_json_typedef_schema(
    schema: serde_json::Value,
    schema_name: &str,
) -> Result<Schema, ComponentLoadErrorInner> {
    let jtd_serde_schema: jtd::SerdeSchema = serde_json::from_value(schema).map_err(|e| {
        ComponentLoadErrorInner::JsonTypeDefParseFailed {
            schema_name: schema_name.to_string(),
            error: Arc::new(e),
        }
    })?;

    let jtd_schema = jtd::Schema::from_serde_schema(jtd_serde_schema).map_err(|e| {
        ComponentLoadErrorInner::JsonTypeDefConversionFailed {
            schema_name: schema_name.to_string(),
            error: e,
        }
    })?;

    Ok(Schema::JsonTypeDef { schema: jtd_schema })
}

fn parse_json_schema(
    mut schema: serde_json::Value,
    schema_name: &str,
    component_files: Arc<ComponentFiles>,
) -> Result<Schema, ComponentLoadErrorInner> {
    if schema.get("$id").is_none() {
        schema["$id"] =
            serde_json::Value::String(format!("{}{}", DEFAULT_BASE_URL_PREFIX, schema_name));
    }

    let compiled_schema = Box::new(
        Validator::options()
            .with_resolver(ComponentJsonSchemaResolver { component_files })
            .build(&schema)
            .map_err(|e| ComponentLoadErrorInner::JsonSchemaParseFailed {
                schema_name: schema_name.to_string(),
                error: e.into(),
            })?,
    );

    Ok(Schema::JsonSchema {
        schema: compiled_schema,
        original: schema,
    })
}

struct ComponentJsonSchemaResolver {
    component_files: Arc<ComponentFiles>,
}

impl SchemaResolver for ComponentJsonSchemaResolver {
    fn resolve(
        &self,
        _root_schema: &serde_json::Value,
        url: &Url,
        _original_reference: &str,
    ) -> Result<Arc<serde_json::Value>, SchemaResolverError> {
        match url.scheme() {
            "http" | "https" => {
                let document: serde_json::Value = ureq::get(url.as_str())
                    .call()
                    .with_context(|| format!("Failed to load schema from {}", url))?
                    .into_body()
                    .read_json()?;
                Ok(Arc::new(document))
            }
            "file" => {
                if let Ok(path) = url.to_file_path() {
                    Ok(self
                        .component_files
                        .get_json(path.to_string_lossy().trim_start_matches('/'))?)
                } else {
                    Err(anyhow::anyhow!(format!(
                        "Invalid absolute file path: {}",
                        url.clone()
                    )))
                }
            }
            "json-schema" => Err(anyhow::anyhow!(
                // Occurs when there is a reference of the form #definitions/blah but
                // there is no schema when an $id from which to resolve it.
                "Cannot resolve relative external schema without root schema ID"
            )),
            _ => Err(anyhow::anyhow!("Unknown scheme: {}", url.scheme())),
        }
    }
}

#[cfg(test)]
mod tests {
    use common_test_utils::test_server::TestServer;
    use jsonschema::{error::ValidationErrorKind, ValidationError};
    use jtd::ValidationErrorIndicator;
    use serde_json::json;
    use std::collections::HashMap;

    use crate::{errors::ComponentLoadError, ComponentFilesLoader, SlipwayReference};

    use super::*;

    struct MockComponentFiles {
        map: HashMap<String, serde_json::Value>,
    }

    fn mock_component_files(map: HashMap<String, serde_json::Value>) -> Arc<ComponentFiles> {
        Arc::new(ComponentFiles::new(Box::new(MockComponentFiles { map })))
    }

    impl ComponentFilesLoader for MockComponentFiles {
        fn get_component_reference(&self) -> &SlipwayReference {
            unimplemented!();
        }

        fn get_component_path(&self) -> &std::path::Path {
            unimplemented!()
        }

        fn exists(&self, _file_name: &str) -> Result<bool, ComponentLoadError> {
            unimplemented!()
        }

        fn try_get_bin(&self, file_name: &str) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
            let json = self
                .map
                .get(file_name)
                .map(|value| Arc::new(value.clone()))
                .ok_or(ComponentLoadError::new(
                    &SlipwayReference::for_test("mock"),
                    ComponentLoadErrorInner::FileLoadFailed {
                        path: file_name.to_string(),
                        error: "file not found in map".to_string(),
                    },
                ))?;

            Ok(Some(Arc::new(serde_json::to_vec(json.as_ref()).unwrap())))
        }

        fn try_get_text(
            &self,
            _file_name: &str,
        ) -> Result<Option<Arc<String>>, ComponentLoadError> {
            unimplemented!()
        }
    }

    #[test]
    fn it_should_parse_json_typedef() {
        let schema = serde_json::json!({
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "uint32" },
                "phones": {
                    "elements": {
                        "type": "string"
                    }
                }
            }
        });

        let component_files = mock_component_files(HashMap::new());

        let input_bad = json!({
            "name": "John Doe",
            "age": "43",
            "phones": ["+44 1234567", "+44 2345678"]
        });

        let schema = parse_schema("test", schema, component_files).unwrap();

        match schema {
            Schema::JsonTypeDef { schema } => {
                assert_eq!(
                    vec![
                        // "age" has the wrong type (required by "/properties/age/type")
                        ValidationErrorIndicator {
                            instance_path: vec!["age".into()],
                            schema_path: vec!["properties".into(), "age".into(), "type".into()],
                        },
                    ],
                    jtd::validate(&schema, &input_bad, Default::default()).unwrap(),
                );
            }
            _ => panic!("expected JsonTypeDef"),
        }
    }

    #[test]
    fn it_should_parse_basic_json_schema() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" },
                "phones": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["name", "age"]
        });

        let component_files = mock_component_files(HashMap::new());

        let schema = parse_schema("test", schema, component_files).unwrap();

        assert_json_schema_errors(schema);
    }

    #[test]
    fn it_should_parse_json_schema_with_file_references() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "properties": {
                "name": { "$ref": "name.json" },
                "age": { "$ref": "age.json" },
                "phones": { "$ref": "phones.json" }
            },
            "required": ["name", "age"]
        });

        let component_files = mock_component_files(HashMap::from([
            ("name.json".to_string(), json!({ "type": "string" })),
            ("age.json".to_string(), json!({ "type": "number" })),
            (
                "phones.json".to_string(),
                json!({ "type": "array", "items": { "type": "string" } }),
            ),
        ]));

        let schema = parse_schema("test", schema, component_files).unwrap();

        assert_json_schema_errors(schema);
    }

    mod serial_tests {
        use super::*;

        #[test]
        fn it_should_parse_json_schema_with_https_references() {
            // Start a server to return the schema.
            let test_server = TestServer::start_from_string_map(HashMap::from([(
                "/name.json".to_string(),
                r#"{ "type": "string" }"#.to_string(),
            )]));

            let schema = serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "properties": {
                    "name": { "$ref": format!("{}name.json", test_server.localhost_url)},
                    "age": { "type": "number" },
                    "phones": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["name", "age"]
            });

            let component_files = mock_component_files(HashMap::new());

            let schema = parse_schema("test", schema, component_files).unwrap();

            assert_json_schema_errors(schema);

            test_server.stop();
        }
    }

    fn create_json_schema_input_bad() -> serde_json::Value {
        let input_bad = json!({
            "name": "John Doe",
            "age": "43",
            "phones": ["+44 1234567", "+44 2345678"]
        });
        input_bad
    }

    fn assert_json_schema_errors(schema: Schema) {
        let input_bad = create_json_schema_input_bad();
        match schema {
            Schema::JsonSchema {
                schema,
                original: _,
            } => {
                let mut errors: Vec<ValidationError> = schema
                    .validate(&input_bad)
                    .map_err(|es| es.into_iter().collect())
                    .unwrap_err();

                println!("{:#?}", errors);

                assert_eq!(errors.len(), 1);

                let error = errors.remove(0);

                match error.kind {
                    ValidationErrorKind::Type { kind: _ } => {}
                    _ => panic!("expected ValidationErrorKind::Type"),
                }

                let instance_path = error.instance_path.into_vec();
                let schema_path = error.schema_path.into_vec();

                assert_eq!(instance_path, vec!["age".to_string()]);
                assert_eq!(
                    schema_path,
                    vec![
                        "properties".to_string(),
                        "age".to_string(),
                        "type".to_string()
                    ]
                );
            }
            _ => panic!("expected JSON Schema"),
        }
    }
}
