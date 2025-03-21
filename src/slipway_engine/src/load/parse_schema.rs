use std::sync::Arc;

use async_trait::async_trait;
use jsonschema::{AsyncRetrieve, Validator};
use thiserror::Error;

use crate::{ComponentFiles, errors::ComponentLoadErrorInner};

use crate::parse::types::Schema;

const DEFAULT_BASE_URL_PREFIX: &str = "file:///";

pub async fn parse_schema(
    schema_name: &str,
    schema: serde_json::Value,
    component_files: Arc<ComponentFiles>,
) -> Result<Schema, ComponentLoadErrorInner> {
    if let Some(serde_json::Value::String(schema_uri)) = schema.get("$schema") {
        if schema_uri.contains("://json-schema.org/") {
            // If the schema contains a $schema field that refers to a JSON Schema
            // then we parse it as a JSON Schema.
            return parse_json_schema(schema, schema_name, component_files).await;
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

async fn parse_json_schema(
    mut schema: serde_json::Value,
    schema_name: &str,
    component_files: Arc<ComponentFiles>,
) -> Result<Schema, ComponentLoadErrorInner> {
    if schema.get("$id").is_none() {
        schema["$id"] =
            serde_json::Value::String(format!("{}{}", DEFAULT_BASE_URL_PREFIX, schema_name));
    }

    let compiled_schema = Box::new(
        Validator::async_options()
            .with_retriever(ComponentJsonSchemaResolver { component_files })
            .build(&schema)
            .await
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

#[derive(Error, Debug)]
pub enum JsonSchemaRetrieveError {
    #[error("Unknown scheme: {scheme}")]
    UnknownScheme { scheme: String },

    // /// Occurs when there is a reference of the form #definitions/blah but
    // /// there is no schema with an $id from which to resolve it.
    // #[error("Cannot resolve relative external schema without root schema ID")]
    // RelativeExternalSchemaWithoutRootSchemaId,
    #[error("Invalid absolute file path: {url}")]
    InvalidAbsoluteFilePath { url: String },

    /// We disallow this because firstly a component shouldn't require an internet connection
    /// just to validate it's input/output, and secondly because what is considered valid
    /// shouldn't change over time, which could happen if it referenced an external schema.
    #[error("Component schemas cannot reference external URLs: {url}")]
    UnsupportedExternalSchemaUrl { url: String },
}

#[async_trait]
impl AsyncRetrieve for ComponentJsonSchemaResolver {
    async fn retrieve(
        &self,
        url: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let scheme = url.scheme().as_str();
        match scheme {
            "http" | "https" => Err(Box::new(
                JsonSchemaRetrieveError::UnsupportedExternalSchemaUrl {
                    url: url.to_string(),
                },
            )),
            "file" | "json-schema" => {
                if let Ok(path) = url::Url::parse(url.as_str())
                    .expect("URL from fluent-uri should be valid")
                    .to_file_path()
                {
                    Ok(self
                        .component_files
                        .get_json(path.to_string_lossy().trim_start_matches('/'))
                        .await
                        // If we remove the Arcs from ComponentFilesLoader we can avoid the clone here.
                        .map(|v: Arc<serde_json::Value>| (*v).clone())?)
                } else {
                    Err(Box::new(JsonSchemaRetrieveError::InvalidAbsoluteFilePath {
                        url: url.to_string(),
                    }))
                }
            }
            _ => Err(Box::new(JsonSchemaRetrieveError::UnknownScheme {
                scheme: scheme.to_string(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use common_macros::slipway_test_async;
    use jsonschema::{ValidationError, error::ValidationErrorKind};
    use jtd::ValidationErrorIndicator;
    use serde_json::json;
    use std::collections::HashMap;

    use crate::errors::JsonSchemaValidationFailure;
    use crate::{ComponentFilesLoader, SlipwayReference, errors::ComponentLoadError};

    use super::*;

    struct MockComponentFiles {
        map: HashMap<String, serde_json::Value>,
    }

    fn mock_component_files(map: HashMap<String, serde_json::Value>) -> Arc<ComponentFiles> {
        Arc::new(ComponentFiles::new(Box::new(MockComponentFiles { map })))
    }

    #[async_trait]
    impl ComponentFilesLoader for MockComponentFiles {
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
            file_name: &str,
        ) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
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

        async fn try_get_text(
            &self,
            _file_name: &str,
        ) -> Result<Option<Arc<String>>, ComponentLoadError> {
            unimplemented!()
        }
    }

    #[slipway_test_async]
    async fn it_should_parse_json_typedef() {
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

        let schema = parse_schema("test", schema, component_files).await.unwrap();

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

    #[slipway_test_async]
    async fn it_should_parse_basic_json_schema() {
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

        let schema = parse_schema("test", schema, component_files).await.unwrap();

        assert_json_schema_errors(schema, false);
    }

    #[slipway_test_async]
    async fn it_should_parse_json_schema_with_file_references() {
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

        let schema = parse_schema("test", schema, component_files).await.unwrap();

        assert_json_schema_errors(schema, true);
    }

    #[slipway_test_async]
    async fn it_should_not_parse_json_schema_with_https_references() {
        // // Start a server to return the schema.
        // let test_server = TestServer::start_from_string_map(HashMap::from([(
        //     "/name.json".to_string(),
        //     r#"{ "type": "string" }"#.to_string(),
        // )]));

        let server_url = "https://localhost:1234/"; // test_server.localhost_url;
        let file_url = format!("{}name.json", server_url);

        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "properties": {
                "name": { "$ref": file_url},
                "age": { "type": "number" },
                "phones": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "required": ["name", "age"]
        });

        let component_files = mock_component_files(HashMap::new());

        let maybe_schema = parse_schema("test", schema, component_files).await;

        match maybe_schema {
            Err(ComponentLoadErrorInner::JsonSchemaParseFailed {
                schema_name: _,
                error:
                    JsonSchemaValidationFailure {
                        kind,
                        instance_path: _,
                        schema_path: _,
                    },
            }) => match &*kind {
                jsonschema::error::ValidationErrorKind::Referencing(
                    jsonschema::ReferencingError::Unretrievable { uri: _, source },
                ) => {
                    let inner_error = source
                        .downcast_ref::<JsonSchemaRetrieveError>()
                        .unwrap_or_else(|| panic!("Unexpected inner error: {:?}", source));
                    match inner_error {
                        JsonSchemaRetrieveError::UnsupportedExternalSchemaUrl { url } => {
                            assert_eq!(url, &file_url);
                        }
                        _ => panic!("expected UnsupportedExternalSchemaUrl"),
                    }
                }
                _ => panic!("expected UriError"),
            },
            _ => panic!("expected JsonSchemaParseFailed"),
        }

        // let schema = parse_schema("test", schema, component_files).await.unwrap();

        // assert_json_schema_errors(schema);

        // test_server.stop();
    }

    fn create_json_schema_input_bad() -> serde_json::Value {
        let input_bad = json!({
            "name": "John Doe",
            "age": "43",
            "phones": ["+44 1234567", "+44 2345678"]
        });
        input_bad
    }

    fn assert_json_schema_errors(schema: Schema, has_ref: bool) {
        let input_bad = create_json_schema_input_bad();
        match schema {
            Schema::JsonSchema {
                schema,
                original: _,
            } => {
                let mut errors: Vec<ValidationError> = schema.iter_errors(&input_bad).collect();

                println!("{:#?}", errors);

                assert_eq!(errors.len(), 1);

                let error = errors.remove(0);

                match error.kind {
                    ValidationErrorKind::Type { kind: _ } => {}
                    _ => panic!("expected ValidationErrorKind::Type"),
                }

                let instance_path = error.instance_path.as_str();
                let schema_path = error.schema_path.as_str();

                assert_eq!(instance_path, "/age");
                assert_eq!(
                    schema_path,
                    if has_ref {
                        "/properties/age/$ref/type"
                    } else {
                        "/properties/age/type"
                    }
                );
            }
            _ => panic!("expected JSON Schema"),
        }
    }
}
