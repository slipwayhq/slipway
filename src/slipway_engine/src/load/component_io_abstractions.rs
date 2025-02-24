use async_trait::async_trait;
use url::Url;

use crate::errors::ComponentLoadErrorInner;

use crate::errors::ComponentLoadError;

use crate::SlipwayReference;

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;

pub(super) trait FileHandle: Read + Seek + Send {}

impl FileHandle for File {}

#[async_trait(?Send)]
pub(super) trait ComponentIOAbstractions: Send + Sync {
    async fn load_text(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<String, ComponentLoadError>;

    async fn load_bin(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Vec<u8>, ComponentLoadError>;

    async fn load_file(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Box<dyn FileHandle>, ComponentLoadError>;

    async fn cache_file_from_url(
        &self,
        url: &Url,
        component_reference: &SlipwayReference,
    ) -> Result<PathBuf, ComponentLoadError>;

    async fn exists(&self, path: &Path) -> bool;

    async fn is_dir(&self, path: &Path) -> bool;
}

#[derive(Clone)]
pub(super) struct ComponentIOAbstractionsImpl {
    local_component_cache_path: PathBuf,
}

impl ComponentIOAbstractionsImpl {
    pub fn new(local_component_cache_path: PathBuf) -> Self {
        Self {
            local_component_cache_path,
        }
    }
}

#[async_trait(?Send)]
impl ComponentIOAbstractions for ComponentIOAbstractionsImpl {
    async fn load_bin(
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

    async fn load_text(
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

    async fn load_file(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Box<dyn FileHandle>, ComponentLoadError> {
        Ok(Box::new(std::fs::File::open(path).map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: path.to_string_lossy().to_string(),
                    error: e.to_string(),
                },
            )
        })?))
    }

    async fn cache_file_from_url(
        &self,
        url: &Url,
        component_reference: &SlipwayReference,
    ) -> Result<PathBuf, ComponentLoadError> {
        let file_name = super::filename_from_url::filename_from_url(url);
        let file_path = self.local_component_cache_path.join(file_name);

        if file_path.exists() {
            return Ok(file_path);
        }

        // Create the directory if it doesn't exist
        if !file_path.parent().unwrap().exists() {
            std::fs::create_dir_all(file_path.parent().unwrap()).map_err(|e| {
                ComponentLoadError::new(
                    component_reference,
                    ComponentLoadErrorInner::FileLoadFailed {
                        path: file_path.to_string_lossy().to_string(),
                        error: format!(
                            "Error creating local components directory at {}.\n{}",
                            self.local_component_cache_path.to_string_lossy(),
                            e
                        ),
                    },
                )
            })?;
        }

        let response = ureq::get(url.as_str()).call().map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: url.to_string(),
                    error: format!("Error fetching component from url.\n{}", e),
                },
            )
        })?;

        if response.status() != 200 {
            return Err(ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: url.to_string(),
                    error: format!(
                        "Unexpected status code downloading component from url.\nHTTP {}",
                        response.status()
                    ),
                },
            ));
        }

        let mut reader = response.into_body().into_reader();
        let mut file = File::create(file_path.clone()).map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: file_path.to_string_lossy().to_string(),
                    error: format!("Error creating file.\n{}", e),
                },
            )
        })?;

        // Stream the response directly to the file
        std::io::copy(&mut reader, &mut file).map_err(|e| {
            ComponentLoadError::new(
                component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: url.to_string(),
                    error: format!(
                        "Error downloading file to {}.\n{}",
                        file_path.to_string_lossy(),
                        e
                    ),
                },
            )
        })?;

        Ok(file_path)
    }

    async fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    async fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}
