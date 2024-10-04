use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use super::component_file_loader::ComponentFileLoader;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::{
        is_safe_path::is_safe_path, SLIPWAY_COMPONENT_FILE_NAME, SLIPWAY_COMPONENT_WASM_FILE_NAME,
    },
    ComponentJson, ComponentWasm, LoadedComponent, SlipwayReference,
};

pub(super) fn load_from_folder<'rig>(
    component_reference: &'rig SlipwayReference,
    path: &Path,
    file_loader: Arc<dyn ComponentFileLoader>,
) -> Result<LoadedComponent<'rig>, ComponentLoadError> {
    let definition_path = path.join(SLIPWAY_COMPONENT_FILE_NAME);
    let definition_string = file_loader.load_text(&definition_path, component_reference)?;

    let wasm_path = path.join(SLIPWAY_COMPONENT_WASM_FILE_NAME);
    let wasm_bytes = file_loader.load_bin(&wasm_path, component_reference)?;
    let component_wasm = Arc::new(InMemoryComponentWasm::new(wasm_bytes));

    let component_json = Arc::new(FolderComponentJson::new(
        file_loader.clone(),
        component_reference.clone(),
        path.to_owned(),
    ));

    Ok(LoadedComponent::<'rig>::new(
        component_reference,
        definition_string,
        component_wasm,
        component_json,
    ))
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
