use async_trait::async_trait;
use tokio::io::AsyncSeekExt;
use tokio_util::io::StreamReader;
use tracing::debug;
use tracing::warn;
use url::Url;

use crate::errors::ComponentLoadErrorInner;

use crate::errors::ComponentLoadError;

use crate::SlipwayReference;

use futures::TryStreamExt;
use std::path::Path;
use std::path::PathBuf;

pub(super) trait FileHandle:
    tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send
{
}

impl FileHandle for tokio::fs::File {}

#[async_trait]
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

#[async_trait]
impl ComponentIOAbstractions for ComponentIOAbstractionsImpl {
    async fn load_bin(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Vec<u8>, ComponentLoadError> {
        tokio::fs::read(path).await.map_err(|e| {
            file_load_failed_error(component_reference, path.to_string_lossy(), e.to_string())
        })
    }

    async fn load_text(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<String, ComponentLoadError> {
        tokio::fs::read_to_string(path).await.map_err(|e| {
            file_load_failed_error(component_reference, path.to_string_lossy(), e.to_string())
        })
    }

    async fn load_file(
        &self,
        path: &Path,
        component_reference: &SlipwayReference,
    ) -> Result<Box<dyn FileHandle>, ComponentLoadError> {
        Ok(Box::new(tokio::fs::File::open(path).await.map_err(
            |e| file_load_failed_error(component_reference, path.to_string_lossy(), e.to_string()),
        )?))
    }

    async fn cache_file_from_url(
        &self,
        url: &Url,
        component_reference: &SlipwayReference,
    ) -> Result<PathBuf, ComponentLoadError> {
        let file_name = super::filename_from_url::filename_from_url(url);
        let file_path = self.local_component_cache_path.join(file_name);

        if file_path.exists() {
            debug!("Found component in cache: {url}");
            return Ok(file_path);
        }

        debug!("Downloading component: {url}");

        // Create the directory if it doesn't exist
        if !file_path.parent().unwrap().exists() {
            tokio::fs::create_dir_all(file_path.parent().unwrap())
                .await
                .map_err(|e| {
                    file_load_failed_error(
                        component_reference,
                        file_path.to_string_lossy(),
                        format!(
                            "Error creating local components directory at {}.\n{e}",
                            self.local_component_cache_path.to_string_lossy(),
                        ),
                    )
                })?;
        }

        let response = reqwest::get(url.as_str()).await.map_err(|e| {
            file_load_failed_error(
                component_reference,
                url,
                format!("Error fetching component from url.\n{e}"),
            )
        })?;

        if response.status() != 200 {
            return Err(file_load_failed_error(
                component_reference,
                url,
                format!(
                    "Unexpected status code downloading component from url.\nHTTP {}",
                    response.status()
                ),
            ));
        }

        let stream = response.bytes_stream();
        let mut reader = StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );

        // We download to a temp file and then move it to the cache to avoid
        // race conditions with other threads trying to download or access the same file.
        let mut temp_file = tokio::fs::File::from_std(tempfile::tempfile().map_err(|e| {
            file_load_failed_error(
                component_reference,
                url,
                format!("Error creating temporary file to download component.\n{e}"),
            )
        })?);

        tokio::io::copy(&mut reader, &mut temp_file)
            .await
            .map_err(|e| {
                file_load_failed_error(
                    component_reference,
                    url,
                    format!(
                        "Error downloading component file to {}.\n{e}",
                        file_path.to_string_lossy()
                    ),
                )
            })?;

        move_temp_file(temp_file, &file_path).await.map_err(|e| {
            file_load_failed_error(
                component_reference,
                file_path.to_string_lossy(),
                format!("Error moving component file to cache.\n{e}"),
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

fn file_load_failed_error(
    component_reference: &SlipwayReference,
    path: impl AsRef<str>,
    error: String,
) -> ComponentLoadError {
    ComponentLoadError::new(
        component_reference,
        ComponentLoadErrorInner::FileLoadFailed {
            path: String::from(path.as_ref()),
            error,
        },
    )
}

async fn move_temp_file(mut tmp_file: tokio::fs::File, file_path: &Path) -> std::io::Result<()> {
    match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)
        .await
    {
        Ok(mut dest_file) => {
            tmp_file.seek(std::io::SeekFrom::Start(0)).await?;
            tokio::io::copy(&mut tmp_file, &mut dest_file).await?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            warn!(
                "Cached component file \"{:?}\" already exists. Existing file will be used.",
                file_path
            );
        }
        Err(e) => return Err(e),
    }

    Ok(())
}
