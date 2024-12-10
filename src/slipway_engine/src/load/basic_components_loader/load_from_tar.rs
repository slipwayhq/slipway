use std::{
    collections::HashMap,
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tar::Archive;

use crate::{
    errors::ComponentLoadError, load::SLIPWAY_COMPONENT_FILE_NAME, ComponentFiles,
    ComponentFilesLoader, LoadedComponent, SlipwayReference,
};

use super::component_io_abstractions::{ComponentIOAbstractions, FileHandle};

type FileEntriesResult = (Box<dyn FileHandle>, HashMap<String, FileEntry>);

pub(super) fn load_from_tar(
    component_reference: &SlipwayReference,
    path: &Path,
    io_abstractions: Arc<dyn ComponentIOAbstractions>,
) -> Result<LoadedComponent, ComponentLoadError> {
    let file: Box<dyn FileHandle> = io_abstractions.load_file(path, component_reference)?;

    let (mut file, all_files) = get_all_file_entries(file, component_reference, path)?;

    let Some(definition_entry) = all_files.get(SLIPWAY_COMPONENT_FILE_NAME) else {
        return Err(ComponentLoadError::new(
            component_reference,
            crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                path: format!("{}:{}", path.to_string_lossy(), SLIPWAY_COMPONENT_FILE_NAME),
                error: format!(
                    "Component TAR file does not contain the definition file \"{}\"",
                    SLIPWAY_COMPONENT_FILE_NAME
                ),
            },
        ));
    };

    let definition_string = map_tar_io_error(
        read_file_string_entry(definition_entry, &mut *file),
        component_reference,
        path,
        SLIPWAY_COMPONENT_FILE_NAME,
        "Failed to read component definition file",
    )?;

    let loader_data = Arc::new(TarComponentFileLoaderData {
        file: Mutex::new(file),
        entries: all_files,
        component_reference: component_reference.clone(),
        path: path.to_owned(),
    });

    let component_files = Arc::new(ComponentFiles::new(Box::new(TarComponentFilesLoader {
        data: loader_data.clone(),
    })));

    Ok(LoadedComponent::new(
        component_reference.clone(),
        definition_string,
        component_files,
    ))
}

struct TarComponentFileLoaderData {
    file: Mutex<Box<dyn FileHandle>>,
    entries: HashMap<String, FileEntry>,
    component_reference: SlipwayReference,
    path: PathBuf,
}

struct TarComponentFilesLoader {
    data: Arc<TarComponentFileLoaderData>,
}

impl ComponentFilesLoader for TarComponentFilesLoader {
    fn get_component_reference(&self) -> &SlipwayReference {
        &self.data.component_reference
    }

    fn get_component_path(&self) -> &Path {
        &self.data.path
    }

    fn get_component_file_separator(&self) -> &str {
        ":"
    }

    fn exists(&self, file_name: &str) -> Result<bool, ComponentLoadError> {
        Ok(self.data.entries.contains_key(file_name))
    }

    fn try_get_bin(&self, file_name: &str) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError> {
        let Some(entry) = self.data.entries.get(file_name) else {
            return Ok(None);
        };

        let mut file = self
            .data
            .file
            .lock()
            .expect("should be able to acquire lock on tar file");

        let data = map_tar_io_error(
            read_file_entry(entry, &mut **file),
            &self.data.component_reference,
            &self.data.path,
            file_name,
            "Failed to read component binary file",
        )?;

        Ok(Some(Arc::new(data)))
    }

    fn try_get_text(&self, file_name: &str) -> Result<Option<Arc<String>>, ComponentLoadError> {
        let Some(entry) = self.data.entries.get(file_name) else {
            return Ok(None);
        };

        let mut file = self
            .data
            .file
            .lock()
            .expect("should be able to acquire lock on tar file");

        let data = map_tar_io_error(
            read_file_string_entry(entry, &mut **file),
            &self.data.component_reference,
            &self.data.path,
            file_name,
            "Failed to read component binary file",
        )?;

        Ok(Some(Arc::new(data)))
    }
}

fn get_all_file_entries(
    file: Box<dyn FileHandle>,
    component_reference: &SlipwayReference,
    path: &Path,
) -> Result<FileEntriesResult, ComponentLoadError> {
    let mut a = Archive::new(file);
    let mut all_files = HashMap::new();
    for file in map_io_error(
        a.entries(),
        component_reference,
        path,
        "Failed to get TAR file entries",
    )? {
        // Make sure there wasn't an I/O error
        let file = map_io_error(
            file,
            component_reference,
            path,
            "Failed to get file handle within TAR file",
        )?;

        // Inspect metadata about the file
        let entry_path = map_io_error(
            file.header().path(),
            component_reference,
            path,
            "Failed to get file entry path",
        )?;

        // Remove the leading "./" from the path if it exists.
        let file_path_raw = entry_path.to_string_lossy().to_string();
        let file_path = match file_path_raw.strip_prefix("./") {
            Some(stripped) => stripped,
            None => &file_path_raw,
        }
        .to_string();

        let length = map_tar_io_error(
            file.header().entry_size(),
            component_reference,
            path,
            &file_path,
            "Failed to get file length",
        )?;
        let offset = file.raw_file_position();

        let file_entry = FileEntry { offset, length };

        all_files.insert(file_path, file_entry);
    }

    let file = a.into_inner();
    Ok((file, all_files))
}

fn map_io_error<T>(
    result: Result<T, std::io::Error>,
    reference: &SlipwayReference,
    path: &Path,
    context: &str,
) -> Result<T, ComponentLoadError> {
    result.map_err(|e| {
        ComponentLoadError::new(
            reference,
            crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                path: path.to_string_lossy().to_string(),
                error: format!("{}: {}", context, e),
            },
        )
    })
}

fn map_tar_io_error<T>(
    result: Result<T, std::io::Error>,
    reference: &SlipwayReference,
    tar_path: &Path,
    inner_path: &str,
    context: &str,
) -> Result<T, ComponentLoadError> {
    result.map_err(|e| {
        ComponentLoadError::new(
            reference,
            crate::errors::ComponentLoadErrorInner::FileLoadFailed {
                path: format!("{}:{}", tar_path.to_string_lossy(), inner_path),
                error: format!("{}: {}", context, e),
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
