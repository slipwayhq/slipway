use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::is_safe_path::is_safe_path,
    SlipwayReference,
};

use super::{ComponentJson, ComponentWasm, ComponentsLoader, LoadedComponent};

pub enum ComponentLoaderErrorBehavior {
    ErrorAlways,
    ErrorIfComponentNotLoaded,
}

pub struct BasicComponentsLoader {
    file_loader: Arc<dyn ComponentFileLoader>,
}

impl BasicComponentsLoader {
    pub fn new() -> Self {
        Self {
            file_loader: Arc::new(ComponentFileLoaderImpl::new()),
        }
    }
}

impl Default for BasicComponentsLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentsLoader for BasicComponentsLoader {
    fn load_components<'rig>(
        &self,
        component_references: &[&'rig SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'rig>, ComponentLoadError>> {
        component_references
            .iter()
            .map(|r| self.load_component(r))
            .collect()
    }
}

impl BasicComponentsLoader {
    fn load_component<'rig>(
        &self,
        component_reference: &'rig SlipwayReference,
    ) -> Result<LoadedComponent<'rig>, ComponentLoadError> {
        match component_reference {
            SlipwayReference::Local { path } => {
                let definition_string = self.file_loader.load_text(path, component_reference)?;

                let wasm_path = path.with_extension("wasm");
                let wasm_bytes = self.file_loader.load_bin(&wasm_path, component_reference)?;
                let component_wasm = Arc::new(InMemoryComponentWasm::new(wasm_bytes));

                let file_path = path.parent().map(|p| p.to_owned()).unwrap_or_else(|| {
                    PathBuf::from_str(".").expect("current directory should be valid path")
                });

                let component_json = Arc::new(FolderComponentJson::new(
                    self.file_loader.clone(),
                    component_reference.clone(),
                    file_path,
                ));

                Ok(LoadedComponent::<'rig>::new(
                    component_reference,
                    definition_string,
                    component_wasm,
                    component_json,
                ))
            }
            _ => unimplemented!("Only local components are supported"),
        }
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
    file_loader: Arc<dyn ComponentFileLoader>,
    component_reference: SlipwayReference,
    folder: PathBuf,
}

impl FolderComponentJson {
    pub fn new(
        file_loader: Arc<dyn ComponentFileLoader>,
        component_reference: SlipwayReference,
        folder: PathBuf,
    ) -> Self {
        Self {
            file_loader,
            component_reference,
            folder,
        }
    }
}

impl ComponentJson for FolderComponentJson {
    fn get(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        fn map_fs_err(
            e: impl ToString,
            path: &str,
            component_reference: &SlipwayReference,
        ) -> ComponentLoadError {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: path.to_string(),
                    error: e.to_string(),
                },
            )
        }

        let file_name = PathBuf::from_str(file_name)
            .map_err(|e| map_fs_err(e, file_name, &self.component_reference))?;

        if file_name.is_absolute() {
            return Err(ComponentLoadError::new(
                &self.component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: file_name.to_string_lossy().to_string(),
                    error: "Absolute paths are not allowed.".to_string(),
                },
            ));
        }

        // Check if the resulting path is inside the folder
        if !is_safe_path(&file_name) {
            return Err(ComponentLoadError::new(
                &self.component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: self.folder.join(file_name).to_string_lossy().to_string(),
                    error: "Only files within the component can be loaded.".to_string(),
                },
            ));
        }

        let path = self.folder.join(file_name);

        let file_contents = self
            .file_loader
            .load_text(&path, &self.component_reference)?;

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

trait ComponentFileLoader: Send + Sync {
    fn load_text(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<String, ComponentLoadError>;
    fn load_bin(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Vec<u8>, ComponentLoadError>;
}

#[derive(Clone)]
struct ComponentFileLoaderImpl {}

impl ComponentFileLoaderImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentFileLoader for ComponentFileLoaderImpl {
    fn load_bin(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Vec<u8>, ComponentLoadError> {
        std::fs::read(path).map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: path.to_string_lossy().to_string(),
                    error: e.to_string(),
                },
            )
        })
    }

    fn load_text(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<String, ComponentLoadError> {
        std::fs::read_to_string(path).map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: path.to_string_lossy().to_string(),
                    error: e.to_string(),
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    struct MockComponentFileLoaderInner {
        text: HashMap<String, String>,
        bin: HashMap<String, Vec<u8>>,
    }

    /// Mock component file loader that returns the contents of files which have been populated.
    struct MockComponentFileLoader {
        map: HashMap<SlipwayReference, MockComponentFileLoaderInner>,
    }

    impl ComponentFileLoader for MockComponentFileLoader {
        fn load_bin(
            &self,
            path: &Path,
            component_reference: &SlipwayReference,
        ) -> Result<Vec<u8>, ComponentLoadError> {
            self.map
                .get(component_reference)
                .unwrap()
                .bin
                .get(path.to_string_lossy().as_ref())
                .ok_or(ComponentLoadError::new(
                    component_reference,
                    ComponentLoadErrorInner::FileLoadFailed {
                        path: path.to_string_lossy().to_string(),
                        error: "Text file not in map".to_string(),
                    },
                ))
                .cloned()
        }

        fn load_text(
            &self,
            path: &Path,
            component_reference: &SlipwayReference,
        ) -> Result<String, ComponentLoadError> {
            self.map
                .get(component_reference)
                .unwrap()
                .text
                .get(path.to_string_lossy().as_ref())
                .ok_or(ComponentLoadError::new(
                    component_reference,
                    ComponentLoadErrorInner::FileLoadFailed {
                        path: path.to_string_lossy().to_string(),
                        error: "Binary file not in map".to_string(),
                    },
                ))
                .cloned()
        }
    }

    #[test]
    fn it_should_load_all_component_files_from_local() {
        let component_reference = SlipwayReference::Local {
            path: PathBuf::from_str("path/to/my_component.json").unwrap(),
        };

        let definition_content = r#"{ "definition": "1" }"#;
        let file1_content = r#"{ "file": "1" }"#;
        let wasm_content = vec![1, 2, 3];

        let file_loader = MockComponentFileLoader {
            map: HashMap::from([(
                component_reference.clone(),
                MockComponentFileLoaderInner {
                    text: HashMap::from([
                        (
                            "path/to/my_component.json".to_string(),
                            definition_content.to_string(),
                        ),
                        ("path/to/file1.json".to_string(), file1_content.to_string()),
                    ]),
                    bin: HashMap::from([(
                        "path/to/my_component.wasm".to_string(),
                        wasm_content.clone(),
                    )]),
                },
            )]),
        };

        let loader = BasicComponentsLoader {
            file_loader: Arc::new(file_loader),
        };

        let result = loader.load_components(&[&component_reference]);

        assert_eq!(result.len(), 1);

        let loaded = result.first().unwrap().as_ref().unwrap();

        assert_eq!(loaded.definition.clone(), definition_content);
        assert_eq!(
            *loaded.json.get("file1.json").unwrap(),
            serde_json::from_str::<serde_json::Value>(file1_content).unwrap()
        );
        assert_eq!(*loaded.wasm.get().unwrap(), wasm_content);

        // Test that loading asking for `file2.json` fails:
        match loaded.json.get("file2.json") {
            Ok(_) => panic!("file2.json should not be found"),
            Err(e) => match e {
                ComponentLoadError {
                    error: ComponentLoadErrorInner::FileLoadFailed { path, .. },
                    ..
                } => {
                    assert_eq!(path, "path/to/file2.json");
                }
                e => panic!("Unexpected error: {:?}", e),
            },
        }
    }

    /// Mock component file loader that always returns the same content for any file.
    struct MockComponentAnyFileLoader {}

    impl ComponentFileLoader for MockComponentAnyFileLoader {
        fn load_bin(
            &self,
            _path: &Path,
            _component_reference: &SlipwayReference,
        ) -> Result<Vec<u8>, ComponentLoadError> {
            Ok(vec![1, 2, 3])
        }

        fn load_text(
            &self,
            _path: &Path,
            _component_reference: &SlipwayReference,
        ) -> Result<String, ComponentLoadError> {
            Ok("{}".to_string())
        }
    }

    #[test]
    fn it_only_allow_file_loading_from_component_directory() {
        let component_reference = SlipwayReference::Local {
            path: PathBuf::from_str("path/to/my_component.json").unwrap(),
        };

        let file_loader = MockComponentAnyFileLoader {};

        let loader = BasicComponentsLoader {
            file_loader: Arc::new(file_loader),
        };

        let result = loader.load_components(&[&component_reference]);

        assert_eq!(result.len(), 1);

        let loaded = result.first().unwrap().as_ref().unwrap();

        assert_eq!(
            *loaded.json.get("file.json").unwrap(),
            serde_json::Value::Object(serde_json::Map::new())
        );

        // Test that loading from an absolute path fails
        match loaded.json.get("/bin/file.json") {
            Ok(_) => panic!("loading absolute file should fail"),
            Err(e) => match e {
                ComponentLoadError {
                    error: ComponentLoadErrorInner::FileLoadFailed { path, .. },
                    ..
                } => {
                    assert_eq!(path, "/bin/file.json");
                }
                e => panic!("Unexpected error: {:?}", e),
            },
        }

        // Test that loading from outside the component fails
        match loaded.json.get("../file.json") {
            Ok(_) => panic!("loading outside component file should fail"),
            Err(e) => match e {
                ComponentLoadError {
                    error: ComponentLoadErrorInner::FileLoadFailed { path, .. },
                    ..
                } => {
                    assert_eq!(path, "path/to/../file.json");
                }
                e => panic!("Unexpected error: {:?}", e),
            },
        }
    }
}
