use std::sync::Arc;

use component_file_loader::{ComponentFileLoader, ComponentFileLoaderImpl};
use url::Url;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    SlipwayReference,
};

use super::{ComponentsLoader, LoadedComponent};

mod component_file_loader;
mod filename_from_url;
mod load_from_directory;
mod load_from_tar;

const DEFAULT_REGISTRY_LOOKUP_URL: &str =
    "https://registry.slipwayhq.com/components/{publisher}/{name}/{version}";

pub enum ComponentLoaderErrorBehavior {
    ErrorAlways,
    ErrorIfComponentNotLoaded,
}

pub struct BasicComponentsLoader {
    registry_lookup_url: Option<String>,
    file_loader: Arc<dyn ComponentFileLoader>,
}

impl BasicComponentsLoader {
    pub fn new() -> Self {
        Self {
            registry_lookup_url: Some(DEFAULT_REGISTRY_LOOKUP_URL.to_string()),
            file_loader: Arc::new(ComponentFileLoaderImpl::new()),
        }
    }

    pub fn for_registry(registry_lookup_url: &str) -> Self {
        Self {
            registry_lookup_url: Some(registry_lookup_url.to_string()),
            file_loader: Arc::new(ComponentFileLoaderImpl::new()),
        }
    }

    pub fn without_registry() -> Self {
        Self {
            registry_lookup_url: None,
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
                if self.file_loader.is_dir(path) {
                    return load_from_directory::load_from_directory(
                        component_reference,
                        path,
                        self.file_loader.clone(),
                    );
                } else if path.extension() == Some("tar".as_ref()) {
                    return load_from_tar::load_from_tar(
                        component_reference,
                        path,
                        self.file_loader.clone(),
                    );
                } else {
                    return Err(ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: path.to_string_lossy().to_string(),
                            error: "Only directories and tar files are supported".to_string(),
                        },
                    ));
                }
            }
            SlipwayReference::Url { url } => {
                let local_path = self
                    .file_loader
                    .load_file_from_url(url, component_reference)?;

                let local_reference = &SlipwayReference::Local { path: local_path };

                let result = self.load_component(local_reference);

                match result {
                    Err(e) => Err(ComponentLoadError::new(component_reference, e.error)),
                    Ok(c) => Ok(LoadedComponent::new(
                        component_reference,
                        c.definition,
                        c.wasm,
                        c.json,
                    )),
                }
            }
            SlipwayReference::Registry {
                publisher,
                name,
                version,
            } => {
                let registry_lookup_url =
                    self.registry_lookup_url
                        .as_ref()
                        .ok_or(ComponentLoadError::new(
                            component_reference,
                            ComponentLoadErrorInner::FileLoadFailed {
                                path: component_reference.to_string(),
                                error: "No registry URL has been set.".to_string(),
                            },
                        ))?;

                let resolved_registry_lookup_url = registry_lookup_url
                    .replace("{publisher}", publisher)
                    .replace("{name}", name)
                    .replace("{version}", &version.to_string());

                let component_url = Url::parse(&resolved_registry_lookup_url).map_err(|e| {
                    ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: resolved_registry_lookup_url,
                            error: format!("Failed to create component URL for registry.\n{}", e),
                        },
                    )
                })?;

                let url_reference = &SlipwayReference::Url { url: component_url };

                let result = self.load_component(url_reference);

                match result {
                    Err(e) => Err(ComponentLoadError::new(component_reference, e.error)),
                    Ok(c) => Ok(LoadedComponent::new(
                        component_reference,
                        c.definition,
                        c.wasm,
                        c.json,
                    )),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        str::FromStr,
    };

    use component_file_loader::FileHandle;

    use super::*;

    mod local_directory {
        use super::*;

        struct MockComponentFileLoaderInner {
            text: HashMap<String, String>,
            bin: HashMap<String, Vec<u8>>,
        }

        /// Mock component file loader that returns the contents of files which have been populated.
        struct MockComponentFileLoader {
            component_path: PathBuf,
            map: HashMap<SlipwayReference, MockComponentFileLoaderInner>,
            url_to_file: HashMap<String, String>,
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

            fn load_file(
                &self,
                _path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Box<dyn FileHandle>, ComponentLoadError> {
                unimplemented!()
            }

            fn load_file_from_url(
                &self,
                url: &Url,
                _component_reference: &SlipwayReference,
            ) -> Result<PathBuf, ComponentLoadError> {
                let file_path_str = self.url_to_file.get(url.as_str()).unwrap();
                Ok(PathBuf::from_str(file_path_str).unwrap())
            }

            fn is_dir(&self, path: &Path) -> bool {
                path == self.component_path
            }
        }

        #[test]
        fn it_should_load_all_component_files_from_local_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component").unwrap(),
            };

            let definition_content = r#"{ "definition": "1" }"#;
            let file1_content = r#"{ "file": "1" }"#;
            let wasm_content = vec![1, 2, 3];

            let file_loader = MockComponentFileLoader {
                component_path: PathBuf::from_str("path/to/my_component").unwrap(),
                url_to_file: HashMap::new(),
                map: HashMap::from([(
                    component_reference.clone(),
                    MockComponentFileLoaderInner {
                        text: HashMap::from([
                            (
                                "path/to/my_component/slipway_component.json".to_string(),
                                definition_content.to_string(),
                            ),
                            (
                                "path/to/my_component/file1.json".to_string(),
                                file1_content.to_string(),
                            ),
                        ]),
                        bin: HashMap::from([(
                            "path/to/my_component/slipway_component.wasm".to_string(),
                            wasm_content.clone(),
                        )]),
                    },
                )]),
            };

            let loader = BasicComponentsLoader {
                registry_lookup_url: None,
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
                        assert_eq!(path, "path/to/my_component/file2.json");
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

            fn load_file(
                &self,
                _path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Box<dyn FileHandle>, ComponentLoadError> {
                unimplemented!()
            }

            fn load_file_from_url(
                &self,
                _url: &Url,
                _component_reference: &SlipwayReference,
            ) -> Result<PathBuf, ComponentLoadError> {
                unimplemented!()
            }

            fn is_dir(&self, _path: &Path) -> bool {
                true
            }
        }

        #[test]
        fn it_only_allow_file_loading_from_component_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component").unwrap(),
            };

            let file_loader = MockComponentAnyFileLoader {};

            let loader = BasicComponentsLoader {
                registry_lookup_url: None,
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
                        assert_eq!(path, "path/to/my_component/../file.json");
                    }
                    e => panic!("Unexpected error: {:?}", e),
                },
            }
        }
    }

    mod local_tar {
        use std::io::Cursor;

        use tar::{Builder, Header};

        use super::*;

        struct MockComponentFileLoader {
            files: HashMap<String, Vec<u8>>,
        }

        impl FileHandle for Cursor<Vec<u8>> {}

        impl ComponentFileLoader for MockComponentFileLoader {
            fn load_text(
                &self,
                _path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<String, ComponentLoadError> {
                unimplemented!();
            }

            fn load_bin(
                &self,
                _path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Vec<u8>, ComponentLoadError> {
                unimplemented!();
            }

            fn load_file(
                &self,
                path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Box<dyn FileHandle>, ComponentLoadError> {
                let data = self.files.get(path.to_string_lossy().as_ref()).unwrap();
                Ok(Box::new(Cursor::new(data.clone())))
            }

            fn load_file_from_url(
                &self,
                _url: &Url,
                _component_reference: &SlipwayReference,
            ) -> Result<PathBuf, ComponentLoadError> {
                unimplemented!();
            }

            fn is_dir(&self, path: &Path) -> bool {
                self.files.iter().all(|(p, _)| p != &path.to_string_lossy())
            }
        }

        fn add_text_to_tar(path: &str, data: &str, builder: &mut Builder<&mut Cursor<Vec<u8>>>) {
            let mut buffer = Cursor::new(data);
            let mut header = Header::new_gnu();
            header.set_size(buffer.get_ref().len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, &mut buffer).unwrap();
        }

        fn add_bin_to_tar(path: &str, data: &[u8], builder: &mut Builder<&mut Cursor<Vec<u8>>>) {
            let mut buffer = Cursor::new(data);
            let mut header = Header::new_gnu();
            header.set_size(buffer.get_ref().len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, &mut buffer).unwrap();
        }

        #[test]
        fn it_should_load_all_component_files_from_local_tar() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component.tar").unwrap(),
            };

            let definition_content = r#"{ "definition": "1" }"#;
            let file1_content = r#"{ "file": "1" }"#;
            let wasm_content = vec![1, 2, 3];

            // Create a tar file in memory
            let mut buffer = Cursor::new(Vec::new());
            {
                let mut builder = Builder::new(&mut buffer);

                add_text_to_tar("slipway_component.json", definition_content, &mut builder);
                add_text_to_tar("file1.json", file1_content, &mut builder);
                add_bin_to_tar("slipway_component.wasm", &wasm_content, &mut builder);

                // Finish writing to the buffer
                builder.finish().unwrap();
            }

            // Now `buffer` contains the entire tar file in memory
            let tar_data = buffer.into_inner();

            let file_loader = MockComponentFileLoader {
                files: HashMap::from([("path/to/my_component.tar".to_string(), tar_data.clone())]),
            };

            let loader = BasicComponentsLoader {
                registry_lookup_url: None,
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
                        assert_eq!(path, "path/to/my_component.tar:file2.json");
                    }
                    e => panic!("Unexpected error: {:?}", e),
                },
            }
        }
    }
}
