use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

use component_io_abstractions::{ComponentIOAbstractions, ComponentIOAbstractionsImpl};
use tracing::debug;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    parse::url::{process_url_str, ProcessedUrl},
    SlipwayReference,
};

use super::{special_components::load_special_component, ComponentsLoader, LoadedComponent};

mod component_io_abstractions;
mod filename_from_url;
mod load_from_directory;
mod load_from_tar;

const DEFAULT_REGISTRY_LOOKUP_URL: &str =
    "https://registry.slipwayhq.com/components/{publisher}/{name}/{version}";

fn get_default_slipway_components_cache_dir() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Home directory required for caching components");
    home_dir.join(".slipway/components")
}

pub struct BasicComponentsLoader {
    registry_lookup_url: Option<String>,
    local_base_directory: PathBuf,
    io_abstractions: Arc<dyn ComponentIOAbstractions>,
}

enum RegistrySelection {
    None,
    Default,
    Custom(String),
}

pub struct BasicComponentsLoaderBuilder {
    registry_lookup_url: RegistrySelection,
    components_cache_path: Option<PathBuf>,
    local_base_directory: Option<PathBuf>,
    io_abstractions: Option<Arc<dyn ComponentIOAbstractions>>,
}

impl BasicComponentsLoaderBuilder {
    pub fn new() -> Self {
        Self {
            registry_lookup_url: RegistrySelection::Default,
            components_cache_path: None,
            local_base_directory: None,
            io_abstractions: None,
        }
    }

    pub fn registry_lookup_url(mut self, url: &str) -> Self {
        self.registry_lookup_url = RegistrySelection::Custom(url.to_string());
        self
    }

    pub fn without_registry(mut self) -> Self {
        self.registry_lookup_url = RegistrySelection::None;
        self
    }

    pub fn components_cache_path(mut self, path: &Path) -> Self {
        self.components_cache_path = Some(path.to_owned());
        self
    }

    pub fn local_base_directory(mut self, path: &Path) -> Self {
        self.local_base_directory = Some(path.to_owned());
        self
    }

    fn io_abstractions(mut self, io_abstractions: Arc<dyn ComponentIOAbstractions>) -> Self {
        self.io_abstractions = Some(io_abstractions);
        self
    }

    pub fn build(self) -> BasicComponentsLoader {
        let registry_lookup_url = match self.registry_lookup_url {
            RegistrySelection::None => None,
            RegistrySelection::Default => Some(DEFAULT_REGISTRY_LOOKUP_URL.to_string()),
            RegistrySelection::Custom(url) => Some(url),
        };

        let components_cache_path = self
            .components_cache_path
            .unwrap_or_else(get_default_slipway_components_cache_dir);

        let local_base_directory = self
            .local_base_directory
            .unwrap_or_else(|| PathBuf::from(""));

        let io_abstractions = self
            .io_abstractions
            .unwrap_or_else(|| Arc::new(ComponentIOAbstractionsImpl::new(components_cache_path)));

        BasicComponentsLoader {
            registry_lookup_url,
            io_abstractions,
            local_base_directory,
        }
    }
}

impl Default for BasicComponentsLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicComponentsLoader {
    pub fn builder() -> BasicComponentsLoaderBuilder {
        BasicComponentsLoaderBuilder::new()
    }
}

impl Default for BasicComponentsLoader {
    fn default() -> Self {
        BasicComponentsLoaderBuilder::new().build()
    }
}

impl ComponentsLoader for BasicComponentsLoader {
    fn load_components(
        &self,
        component_references: &[SlipwayReference],
    ) -> Vec<Result<LoadedComponent, ComponentLoadError>> {
        component_references
            .iter()
            .map(|r| self.load_component(r))
            .collect()
    }
}

impl BasicComponentsLoader {
    fn load_component(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<LoadedComponent, ComponentLoadError> {
        debug!("Loading component: {}", component_reference);
        match component_reference {
            SlipwayReference::Special(inner) => Ok(load_special_component(inner)),
            SlipwayReference::Local { path } => {
                let path = if path.is_relative() {
                    Cow::Owned(self.local_base_directory.join(path))
                } else {
                    Cow::Borrowed(path)
                };

                if self.io_abstractions.is_dir(&path) {
                    load_from_directory::load_from_directory(
                        component_reference,
                        &path,
                        Arc::clone(&self.io_abstractions),
                    )
                } else if path.extension() == Some("tar".as_ref()) {
                    load_from_tar::load_from_tar(
                        component_reference,
                        &path,
                        Arc::clone(&self.io_abstractions),
                    )
                } else {
                    Err(ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: path.to_string_lossy().to_string(),
                            error: "Only directories and tar files are supported".to_string(),
                        },
                    ))
                }
            }
            SlipwayReference::Http { url } => {
                let local_path = self
                    .io_abstractions
                    .load_file_from_url(url, component_reference)?;

                let local_reference = SlipwayReference::Local { path: local_path };

                let result = self.load_component(&local_reference);

                match result {
                    Err(e) => Err(ComponentLoadError::new(component_reference, e.error)),
                    Ok(c) => Ok(LoadedComponent::new(
                        component_reference.clone(),
                        c.definition,
                        c.files,
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

                let processed_url =
                    process_url_str(&resolved_registry_lookup_url).map_err(|e| {
                        ComponentLoadError::new(
                            component_reference,
                            ComponentLoadErrorInner::FileLoadFailed {
                                path: resolved_registry_lookup_url,
                                error: format!(
                                    "Failed to create component URL for registry.\n{}",
                                    e
                                ),
                            },
                        )
                    })?;

                let url_reference = match processed_url {
                    ProcessedUrl::RelativePath(path) => SlipwayReference::Local { path },
                    ProcessedUrl::AbsolutePath(path) => SlipwayReference::Local { path },
                    ProcessedUrl::Http(url) => SlipwayReference::Http { url },
                };

                let result = self.load_component(&url_reference);

                match result {
                    Err(e) => Err(ComponentLoadError::new(component_reference, e.error)),
                    Ok(c) => Ok(LoadedComponent::new(
                        component_reference.clone(),
                        c.definition,
                        c.files,
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

    use component_io_abstractions::FileHandle;

    use super::*;

    mod local_directory {
        use std::ffi::OsStr;

        use url::Url;

        use super::*;

        struct MockComponentFileLoaderInner {
            text: HashMap<String, String>,
            bin: HashMap<String, Vec<u8>>,
        }

        /// Mock component file loader that returns the contents of files which have been populated.
        struct MockComponentIOAbstractions {
            component_path: PathBuf,
            component_reference: SlipwayReference,
            map: MockComponentFileLoaderInner,
            url_to_file: HashMap<String, String>,
        }

        impl ComponentIOAbstractions for MockComponentIOAbstractions {
            fn load_bin(
                &self,
                path: &Path,
                component_reference: &SlipwayReference,
            ) -> Result<Vec<u8>, ComponentLoadError> {
                println!("load_bin: {:?}", path);
                assert_eq!(component_reference, &self.component_reference);
                let maybe_bin = self.map.bin.get(path.to_string_lossy().as_ref());

                if let Some(bin) = maybe_bin {
                    return Ok(bin.clone());
                }

                // Reading JSON files can call `load_bin`, so search text files as well.
                self.map
                    .text
                    .get(path.to_string_lossy().as_ref())
                    .map(|s| s.as_bytes().to_vec())
                    .ok_or(ComponentLoadError::new(
                        component_reference,
                        ComponentLoadErrorInner::FileLoadFailed {
                            path: path.to_string_lossy().to_string(),
                            error: "Text file not in map".to_string(),
                        },
                    ))
            }

            fn load_text(
                &self,
                path: &Path,
                component_reference: &SlipwayReference,
            ) -> Result<String, ComponentLoadError> {
                println!("load_text: {:?}", path);
                assert_eq!(component_reference, &self.component_reference);
                self.map
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

            fn exists(&self, path: &Path) -> bool {
                self.map.bin.contains_key(path.to_string_lossy().as_ref())
                    || self.map.text.contains_key(path.to_string_lossy().as_ref())
            }

            fn is_dir(&self, path: &Path) -> bool {
                path == self.component_path
            }
        }

        #[test]
        fn it_should_load_all_component_files_from_relative_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component").unwrap(),
            };

            run_load_all_component_files_tests(component_reference, "path/to/my_component", None);
        }

        #[test]
        fn it_should_load_all_component_files_from_relative_to_base_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component").unwrap(),
            };

            run_load_all_component_files_tests(
                component_reference,
                "some/other/path/to/my_component",
                Some("some/other"),
            );
        }

        #[test]
        fn it_should_load_all_component_files_from_relative_to_absolute_base_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component").unwrap(),
            };

            run_load_all_component_files_tests(
                component_reference,
                "/some/other/path/to/my_component",
                Some("/some/other"),
            );
        }

        #[test]
        fn it_should_load_all_component_files_from_absolute_directory() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("/path/to/my_component").unwrap(),
            };

            run_load_all_component_files_tests(
                component_reference,
                "/path/to/my_component",
                Some("/some/other"),
            );
        }

        fn run_load_all_component_files_tests(
            component_reference: SlipwayReference,
            path_to_component: &str,
            local_base_directory: Option<&str>,
        ) {
            let definition_content = r#"{ "definition": "1" }"#;
            let file1_content = r#"{ "file": "1" }"#;
            let binary_content = vec![1, 2, 3];

            let io_abstractions = MockComponentIOAbstractions {
                component_path: PathBuf::from_str(path_to_component).unwrap(),
                component_reference: component_reference.clone(),
                url_to_file: HashMap::new(),
                map: MockComponentFileLoaderInner {
                    text: HashMap::from([
                        (
                            format!("{}/slipway_component.json", path_to_component),
                            definition_content.to_string(),
                        ),
                        (
                            format!("{}/file1.json", path_to_component),
                            file1_content.to_string(),
                        ),
                    ]),
                    bin: HashMap::from([(
                        format!("{}/bin_file.bin", path_to_component),
                        binary_content.clone(),
                    )]),
                },
            };

            let mut loader_builder =
                BasicComponentsLoaderBuilder::new().io_abstractions(Arc::new(io_abstractions));

            if let Some(local_base_directory) = local_base_directory {
                loader_builder =
                    loader_builder.local_base_directory(&PathBuf::from(local_base_directory));
            }

            let loader = loader_builder.build();

            let result = loader.load_components(&[component_reference]);

            assert_eq!(result.len(), 1);

            let loaded = result.first().unwrap().as_ref().unwrap();

            assert_eq!(loaded.definition.clone(), definition_content);
            assert_eq!(
                *loaded
                    .files
                    .get_json::<serde_json::Value>("file1.json")
                    .unwrap(),
                serde_json::from_str::<serde_json::Value>(file1_content).unwrap()
            );
            assert_eq!(
                *loaded.files.get_bin("bin_file.bin").unwrap(),
                binary_content
            );

            // Test that loading asking for `file2.json` fails:
            match loaded.files.get_json::<serde_json::Value>("file2.json") {
                Ok(_) => panic!("file2.json should not be found"),
                Err(e) => match e {
                    ComponentLoadError {
                        error: ComponentLoadErrorInner::FileLoadFailed { path, .. },
                        ..
                    } => {
                        assert_eq!(path, format!("{}/file2.json", path_to_component));
                    }
                    e => panic!("Unexpected error: {:?}", e),
                },
            }
        }

        /// Mock component file loader that always returns the same content for any file.
        struct MockComponentAnyFileIOAbstractions {}

        impl ComponentIOAbstractions for MockComponentAnyFileIOAbstractions {
            fn load_bin(
                &self,
                path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Vec<u8>, ComponentLoadError> {
                println!("load_bin: {:?}", path);
                if path.extension() == Some(OsStr::new("json")) {
                    Ok("{}".as_bytes().to_vec())
                } else {
                    Ok(vec![1, 2, 3])
                }
            }

            fn load_text(
                &self,
                path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<String, ComponentLoadError> {
                println!("load_text: {:?}", path);
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

            fn exists(&self, _path: &Path) -> bool {
                true
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

            let io_abstractions = MockComponentAnyFileIOAbstractions {};

            let loader = BasicComponentsLoaderBuilder::new()
                .io_abstractions(Arc::new(io_abstractions))
                .build();

            let result = loader.load_components(&[component_reference]);

            assert_eq!(result.len(), 1);

            let loaded = result.first().unwrap().as_ref().unwrap();

            assert_eq!(
                *loaded
                    .files
                    .get_json::<serde_json::Value>("file.json")
                    .unwrap(),
                serde_json::Value::Object(serde_json::Map::new())
            );

            // Test that loading from an absolute path fails
            match loaded.files.get_json::<serde_json::Value>("/bin/file.json") {
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
            match loaded.files.get_json::<serde_json::Value>("../file.json") {
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

    mod local_and_remote_tar {
        use std::io::Cursor;

        use semver::Version;
        use tar::{Builder, Header};
        use url::Url;

        use super::*;

        struct MockComponentIOAbstractions {
            files: HashMap<String, Vec<u8>>,
            url_to_file_map: HashMap<String, String>,
        }

        impl FileHandle for Cursor<Vec<u8>> {}

        impl ComponentIOAbstractions for MockComponentIOAbstractions {
            fn load_text(
                &self,
                path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<String, ComponentLoadError> {
                println!("load_text: {:?}", path);
                unimplemented!();
            }

            fn load_bin(
                &self,
                path: &Path,
                _component_reference: &SlipwayReference,
            ) -> Result<Vec<u8>, ComponentLoadError> {
                println!("load_bin: {:?}", path);
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
                Ok(self
                    .url_to_file_map
                    .get(_url.as_str())
                    .map(|s| PathBuf::from_str(s).unwrap())
                    .unwrap())
            }

            fn exists(&self, path: &Path) -> bool {
                self.files.contains_key(path.to_string_lossy().as_ref())
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

        struct MockData {
            definition_content: &'static str,
            file1_content: &'static str,
            bin_content: Vec<u8>,
        }

        impl MockData {
            fn new() -> Self {
                Self {
                    definition_content: r#"{ "definition": "1" }"#,
                    file1_content: r#"{ "file": "1" }"#,
                    bin_content: vec![1, 2, 3],
                }
            }
        }

        fn create_tar(data: &MockData) -> Vec<u8> {
            // Create a tar file in memory
            let mut buffer = Cursor::new(Vec::new());
            {
                let mut builder = Builder::new(&mut buffer);

                add_text_to_tar(
                    "slipway_component.json",
                    data.definition_content,
                    &mut builder,
                );
                add_text_to_tar("file1.json", data.file1_content, &mut builder);
                add_bin_to_tar("bin_file.bin", &data.bin_content, &mut builder);

                // Finish writing to the buffer
                builder.finish().unwrap();
            }

            // Now `buffer` contains the entire tar file in memory
            buffer.into_inner()
        }

        fn assert_result(
            loader: BasicComponentsLoader,
            component_reference: SlipwayReference,
            data: MockData,
            expected_component_path: &str,
        ) {
            let result = loader.load_components(&[component_reference]);

            assert_eq!(result.len(), 1);

            let loaded = result.first().unwrap().as_ref().unwrap();

            assert_eq!(loaded.definition.clone(), data.definition_content);
            assert_eq!(
                *loaded
                    .files
                    .get_json::<serde_json::Value>("file1.json")
                    .unwrap(),
                serde_json::from_str::<serde_json::Value>(data.file1_content).unwrap()
            );
            assert_eq!(
                *loaded.files.get_bin("bin_file.bin").unwrap(),
                data.bin_content
            );

            // Test that loading asking for `file2.json` fails:
            match loaded.files.get_json::<serde_json::Value>("file2.json") {
                Ok(_) => panic!("file2.json should not be found"),
                Err(e) => match e {
                    ComponentLoadError {
                        error: ComponentLoadErrorInner::FileLoadFailed { path, .. },
                        ..
                    } => {
                        assert_eq!(path, format!("{}:file2.json", expected_component_path));
                    }
                    e => panic!("Unexpected error: {:?}", e),
                },
            }
        }

        #[test]
        fn it_should_load_all_component_files_from_relative_tar() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component.tar").unwrap(),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([("path/to/my_component.tar".to_string(), tar_data.clone())]),
                url_to_file_map: HashMap::new(),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .io_abstractions(Arc::new(io_abstractions))
                .build();

            assert_result(
                loader,
                component_reference,
                data,
                "path/to/my_component.tar",
            );
        }

        #[test]
        fn it_should_load_all_component_files_from_relative_to_base_tar() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("path/to/my_component.tar").unwrap(),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([(
                    "some/other/path/to/my_component.tar".to_string(),
                    tar_data.clone(),
                )]),
                url_to_file_map: HashMap::new(),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .io_abstractions(Arc::new(io_abstractions))
                .local_base_directory(Path::new("some/other"))
                .build();

            assert_result(
                loader,
                component_reference,
                data,
                "some/other/path/to/my_component.tar",
            );
        }

        #[test]
        fn it_should_load_all_component_files_from_absolute_tar() {
            let component_reference = SlipwayReference::Local {
                path: PathBuf::from_str("/path/to/my_component.tar").unwrap(),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([("/path/to/my_component.tar".to_string(), tar_data.clone())]),
                url_to_file_map: HashMap::new(),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .io_abstractions(Arc::new(io_abstractions))
                .local_base_directory(Path::new("some/other"))
                .build();

            assert_result(
                loader,
                component_reference,
                data,
                "/path/to/my_component.tar",
            );
        }

        #[test]
        fn it_should_load_from_url() {
            // This test does not test the actual downloading of the file, but rather the loading
            // of the tar file once it has been downloaded.
            const URL: &str = "http://example.com/path/to/my_component.tar";
            let component_reference = SlipwayReference::Http {
                url: Url::parse(URL).unwrap(),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([("path/to/my_component.tar".to_string(), tar_data.clone())]),
                url_to_file_map: HashMap::from([(
                    URL.to_string(),
                    "path/to/my_component.tar".to_string(),
                )]),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .io_abstractions(Arc::new(io_abstractions))
                .build();

            assert_result(
                loader,
                component_reference,
                data,
                "path/to/my_component.tar",
            );
        }

        #[test]
        fn it_should_load_from_registry() {
            // This test does not test the actual downloading of the file, but rather the loading
            // of the tar file once it has been downloaded.
            const URL: &str = "http://example.com/path/to/{publisher}.{name}.{version}.tar";
            let component_reference = SlipwayReference::Registry {
                publisher: "p1".to_string(),
                name: "n1".to_string(),
                version: Version::parse("1.2.3").expect("Invalid version"),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([("path/to/my_component.tar".to_string(), tar_data.clone())]),
                url_to_file_map: HashMap::from([(
                    "http://example.com/path/to/p1.n1.1.2.3.tar".to_string(),
                    "path/to/my_component.tar".to_string(),
                )]),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .registry_lookup_url(URL)
                .io_abstractions(Arc::new(io_abstractions))
                .build();

            assert_result(
                loader,
                component_reference,
                data,
                "path/to/my_component.tar",
            );
        }

        #[test]
        fn it_should_load_from_local_registry() {
            const URL: &str = "file:path/to/{publisher}.{name}.{version}.tar";
            let component_reference = SlipwayReference::Registry {
                publisher: "p1".to_string(),
                name: "n1".to_string(),
                version: Version::parse("1.2.3").expect("Invalid version"),
            };

            let data = MockData::new();
            let tar_data = create_tar(&data);

            let io_abstractions = MockComponentIOAbstractions {
                files: HashMap::from([("path/to/p1.n1.1.2.3.tar".to_string(), tar_data.clone())]),
                url_to_file_map: HashMap::new(),
            };

            let loader = BasicComponentsLoaderBuilder::new()
                .registry_lookup_url(URL)
                .io_abstractions(Arc::new(io_abstractions))
                .build();

            assert_result(loader, component_reference, data, "path/to/p1.n1.1.2.3.tar");
        }
    }
}
