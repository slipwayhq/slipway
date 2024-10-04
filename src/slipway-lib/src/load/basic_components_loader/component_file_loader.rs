use crate::errors::ComponentLoadErrorInner;

use crate::errors::ComponentLoadError;

use crate::SlipwayReference;

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;

pub(super) trait FileHandle: Read + Seek + Send {}

impl FileHandle for File {}

pub(super) trait ComponentFileLoader: Send + Sync {
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

    fn load_file(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Box<dyn FileHandle>, ComponentLoadError>;

    fn is_dir(&self, path: &Path) -> bool;
}

#[derive(Clone)]
pub(super) struct ComponentFileLoaderImpl {}

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

    fn load_file(
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

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}
