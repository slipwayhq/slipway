use std::{path::PathBuf, str::FromStr, sync::Arc};

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    SlipwayReference,
};

use super::{ComponentJson, ComponentWasm, ComponentsLoader, LoadedComponent};

pub enum ComponentLoaderErrorBehavior {
    ErrorAlways,
    ErrorIfComponentNotLoaded,
}

#[derive(Default)]
pub struct BasicComponentsLoader {}

impl BasicComponentsLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsLoader for BasicComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>> {
        component_references
            .iter()
            .map(|r| load_component(r))
            .collect()
    }
}

fn load_component(
    component_reference: &SlipwayReference,
) -> Result<LoadedComponent, ComponentLoadError> {
    match component_reference {
        SlipwayReference::Local { path } => {
            let definition_string = std::fs::read_to_string(path).map_err(|e| {
                ComponentLoadError::new(
                    component_reference,
                    ComponentLoadErrorInner::DefinitionLoadFailed {
                        error: e.to_string(),
                    },
                )
            })?;

            let wasm_path = path.with_extension("wasm");
            let wasm_bytes = std::fs::read(wasm_path).map_err(|e| {
                ComponentLoadError::new(
                    component_reference,
                    ComponentLoadErrorInner::WasmLoadFailed {
                        error: e.to_string(),
                    },
                )
            })?;
            let component_wasm = Box::new(InMemoryComponentWasm::new(wasm_bytes));

            let file_path = path.parent().map(|p| p.to_owned()).unwrap_or_else(|| {
                PathBuf::from_str(".").expect("current directory should be valid path")
            });

            let component_json = Box::new(FolderComponentJson::new(
                component_reference.clone(),
                file_path,
            ));

            Ok(LoadedComponent::new(
                component_reference,
                definition_string,
                component_wasm,
                component_json,
            ))
        }
        _ => unimplemented!("Only local components are supported"),
    }
}

struct InMemoryComponentWasm {
    wasm: Arc<Vec<u8>>,
}

impl InMemoryComponentWasm {
    pub fn new(wasm: Vec<u8>) -> Self {
        Self {
            wasm: Arc::new(wasm),
        }
    }
}

impl ComponentWasm for InMemoryComponentWasm {
    fn get(&self) -> Result<Arc<Vec<u8>>, ComponentLoadError> {
        Ok(self.wasm.clone())
    }
}

struct FolderComponentJson {
    component_reference: SlipwayReference,
    folder: std::path::PathBuf,
}

impl FolderComponentJson {
    pub fn new(component_reference: SlipwayReference, folder: std::path::PathBuf) -> Self {
        Self {
            component_reference,
            folder,
        }
    }
}

impl ComponentJson for FolderComponentJson {
    fn get(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        let path = self.folder.join(file_name);
        let file_contents = std::fs::read_to_string(path.clone()).map_err(|e| {
            ComponentLoadError::new(
                &self.component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: path.clone(),
                    error: e.to_string(),
                },
            )
        })?;

        let json: serde_json::Value = serde_json::from_str(&file_contents).map_err(|e| {
            ComponentLoadError::new(
                &self.component_reference,
                ComponentLoadErrorInner::FileJsonParseFailed {
                    path: path.clone(),
                    error: Arc::new(e),
                },
            )
        })?;

        Ok(Arc::new(json))
    }
}
