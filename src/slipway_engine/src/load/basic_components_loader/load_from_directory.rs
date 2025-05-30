use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use async_trait::async_trait;

use super::super::component_io_abstractions::ComponentIOAbstractions;

use crate::{
    ComponentFiles, ComponentFilesLoader, LoadedComponent, SlipwayReference,
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::{SLIPWAY_COMPONENT_FILE_NAME, is_safe_path::is_safe_path},
};

pub(super) async fn load_from_directory(
    component_reference: &SlipwayReference,
    path: &Path,
    io_abstractions: Arc<dyn ComponentIOAbstractions>,
) -> Result<LoadedComponent, ComponentLoadError> {
    let definition_path = path.join(SLIPWAY_COMPONENT_FILE_NAME);
    let definition_string = io_abstractions
        .load_text(&definition_path, component_reference)
        .await?;

    let component_files = Arc::new(ComponentFiles::new(Box::new(
        DirectoryComponentFilesLoader::new(
            io_abstractions.clone(),
            component_reference.clone(),
            path.to_owned(),
        ),
    )));

    Ok(LoadedComponent::new(
        component_reference.clone(),
        definition_string,
        component_files,
    ))
}

struct DirectoryComponentFilesLoader {
    io_abstractions: Arc<dyn ComponentIOAbstractions>,
    component_reference: SlipwayReference,
    directory: PathBuf,
}

impl DirectoryComponentFilesLoader {
    pub fn new(
        io_abstractions: Arc<dyn ComponentIOAbstractions>,
        component_reference: SlipwayReference,
        directory: PathBuf,
    ) -> Self {
        Self {
            io_abstractions,
            component_reference,
            directory,
        }
    }

    fn get_valid_file_path(&self, file_name: &str) -> Result<PathBuf, ComponentLoadError> {
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

        if !is_safe_path(&file_name) {
            return Err(ComponentLoadError::new(
                &self.component_reference,
                ComponentLoadErrorInner::FileLoadFailed {
                    path: self.directory.join(file_name).to_string_lossy().to_string(),
                    error: "Only files within the component can be loaded.".to_string(),
                },
            ));
        }

        let path = self.directory.join(file_name);

        Ok(path)
    }
}

#[async_trait]
impl ComponentFilesLoader for DirectoryComponentFilesLoader {
    fn get_component_reference(&self) -> &SlipwayReference {
        &self.component_reference
    }

    fn get_component_path(&self) -> &Path {
        &self.directory
    }

    async fn exists(&self, file_name: &str) -> Result<bool, ComponentLoadError> {
        let path = self.get_valid_file_path(file_name)?;
        Ok(self.io_abstractions.exists(&path).await)
    }

    async fn try_get_bin(
        &self,
        file_name: &str,
    ) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        let path = self.get_valid_file_path(file_name)?;

        if !self.io_abstractions.exists(&path).await {
            return Ok(None);
        }

        let file_contents = self
            .io_abstractions
            .load_bin(&path, &self.component_reference)
            .await?;

        Ok(Some(Arc::new(file_contents)))
    }

    async fn try_get_text(
        &self,
        file_name: &str,
    ) -> Result<Option<Arc<String>>, ComponentLoadError> {
        let path = self.get_valid_file_path(file_name)?;

        if !self.io_abstractions.exists(&path).await {
            return Ok(None);
        }

        let file_contents = self
            .io_abstractions
            .load_text(&path, &self.component_reference)
            .await?;

        Ok(Some(Arc::new(file_contents)))
    }
}

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
