use std::{
    collections::HashMap,
    io::{Read, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tar::Archive;

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    load::{SLIPWAY_COMPONENT_FILE_NAME, SLIPWAY_COMPONENT_WASM_FILE_NAME},
    ComponentJson, ComponentWasm, LoadedComponent, SlipwayReference,
};

use super::component_file_loader::{ComponentFileLoader, FileHandle};

type FileEntriesResult = (Box<dyn FileHandle>, HashMap<String, FileEntry>);

pub(super) fn load_from_tar<'rig>(
    component_reference: &'rig SlipwayReference,
    path: &Path,
    file_loader: Arc<dyn ComponentFileLoader>,
) -> Result<LoadedComponent<'rig>, ComponentLoadError> {
    let file: Box<dyn FileHandle> = file_loader.load_file(path, component_reference)?;

    let (mut file, all_files) = get_all_file_entries(file, component_reference, path)?;

    let Some(definition_entry) = all_files.get(SLIPWAY_COMPONENT_FILE_NAME) else {
        return Err(ComponentLoadError::new(
            component_reference,
            crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                path: path.to_string_lossy().to_string(),
                error: format!(
                    "Component TAR file does not contain the definition file \"{}\"",
                    SLIPWAY_COMPONENT_FILE_NAME
                ),
            },
        ));
    };

    let definition_string = map_io_error(
        read_file_string_entry(definition_entry, &mut *file),
        component_reference,
        path,
    )?;

    let loader_data = Arc::new(TarComponentFileLoaderData {
        file: Mutex::new(file),
        entries: all_files,
        component_reference: component_reference.clone(),
        path: path.to_owned(),
    });

    let component_wasm = Arc::new(TarComponentWasm {
        data: loader_data.clone(),
    });

    let component_json = Arc::new(TarComponentJson {
        data: loader_data.clone(),
    });

    Ok(LoadedComponent::<'rig>::new(
        component_reference,
        definition_string,
        component_wasm,
        component_json,
    ))
}

struct TarComponentFileLoaderData {
    file: Mutex<Box<dyn FileHandle>>,
    entries: HashMap<String, FileEntry>,
    component_reference: SlipwayReference,
    path: PathBuf,
}

struct TarComponentWasm {
    data: Arc<TarComponentFileLoaderData>,
}

impl ComponentWasm for TarComponentWasm {
    fn get(&self) -> Result<Arc<Vec<u8>>, ComponentLoadError> {
        let Some(wasm_entry) = self.data.entries.get(SLIPWAY_COMPONENT_WASM_FILE_NAME) else {
            return Err(ComponentLoadError::new(
                &self.data.component_reference,
                crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                    path: self.data.path.to_string_lossy().to_string(),
                    error: format!(
                        "Component TAR file does not contain the WASM file \"{}\"",
                        SLIPWAY_COMPONENT_WASM_FILE_NAME
                    ),
                },
            ));
        };

        let mut file = self
            .data
            .file
            .lock()
            .expect("should be able to acquire lock on tar file");

        let wasm = map_io_error(
            read_file_entry(wasm_entry, &mut **file),
            &self.data.component_reference,
            &self.data.path,
        )?;

        Ok(Arc::new(wasm))
    }
}

struct TarComponentJson {
    data: Arc<TarComponentFileLoaderData>,
}

impl ComponentJson for TarComponentJson {
    fn get(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        let Some(entry) = self.data.entries.get(file_name) else {
            return Err(ComponentLoadError::new(
                &self.data.component_reference,
                crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                    path: self.data.path.to_string_lossy().to_string(),
                    error: format!(
                        "Component TAR file does not contain the file \"{}\"",
                        file_name
                    ),
                },
            ));
        };

        let mut file = self
            .data
            .file
            .lock()
            .expect("should be able to acquire lock on tar file");

        let buffer = map_io_error(
            read_file_entry(entry, &mut **file),
            &self.data.component_reference,
            &self.data.path,
        )?;

        let json = serde_json::from_slice(&buffer).map_err(|e| {
            ComponentLoadError::new(
                &self.data.component_reference,
                ComponentLoadErrorInner::FileJsonParseFailed {
                    path: self.data.path.clone(),
                    error: Arc::new(e),
                },
            )
        })?;

        Ok(Arc::new(json))
    }
}

fn get_all_file_entries(
    file: Box<dyn FileHandle>,
    component_reference: &SlipwayReference,
    path: &Path,
) -> Result<FileEntriesResult, ComponentLoadError> {
    let mut a = Archive::new(file);
    let mut all_files = HashMap::new();
    for file in map_io_error(a.entries(), component_reference, path)? {
        // Make sure there wasn't an I/O error
        let mut file = map_io_error(file, component_reference, path)?;

        // Inspect metadata about the file
        let entry_path = map_io_error(file.header().path(), component_reference, path)?;
        let length = map_io_error(file.header().entry_size(), component_reference, path)?;
        let offset = file.raw_file_position();

        let file_entry = FileEntry { offset, length };

        all_files.insert(entry_path.to_string_lossy().to_string(), file_entry);

        // files implement the Read trait
        let mut s = String::new();
        map_io_error(file.read_to_string(&mut s), component_reference, path)?;
        println!("{}", s);
    }

    let file = a.into_inner();
    Ok((file, all_files))
}

fn map_io_error<T>(
    result: Result<T, std::io::Error>,
    reference: &SlipwayReference,
    path: &Path,
) -> Result<T, ComponentLoadError> {
    result.map_err(|e| {
        ComponentLoadError::new(
            reference,
            crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                path: path.to_string_lossy().to_string(),
                error: e.to_string(),
            },
        )
    })
}

struct FileEntry {
    offset: u64,
    length: u64,
}

fn read_file<R: FileHandle>(
    name: &str,
    file: &mut R,
    entries: &HashMap<String, FileEntry>,
) -> Result<Vec<u8>, std::io::Error> {
    let entry = entries.get(name).expect("Entry is not in archive");
    read_file_entry(entry, file)
}

fn read_file_as_string<R: FileHandle>(
    name: &str,
    file: &mut R,
    entries: &HashMap<String, FileEntry>,
) -> Result<String, std::io::Error> {
    let entry = entries.get(name).expect("Entry is not in archive");
    let buffer = read_file_entry(entry, file)?;
    Ok(String::from_utf8(buffer).expect("File is not valid UTF-8"))
}

fn read_file_string_entry(
    entry: &FileEntry,
    file: &mut dyn FileHandle,
) -> Result<String, std::io::Error> {
    let buffer = read_file_entry(entry, file)?;
    Ok(String::from_utf8(buffer).expect("File is not valid UTF-8"))
}

fn read_file_entry(
    entry: &FileEntry,
    file: &mut dyn FileHandle,
) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = vec![0; entry.length as usize];
    file.seek(SeekFrom::Start(entry.offset))?;
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}
