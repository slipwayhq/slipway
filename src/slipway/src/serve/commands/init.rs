use std::path::{Path, PathBuf};

use tracing::info;

use crate::serve::{RepositoryConfig, SlipwayServeConfig, get_serve_config_path};

pub async fn init(serve_path: PathBuf) -> anyhow::Result<()> {
    init_serve_config(&serve_path).await?;
    init_dockerfile(&serve_path).await?;

    init_folder(&serve_path, super::COMPONENTS_PATH).await?;
    init_folder(
        &serve_path,
        crate::serve::repository::file_system::RIG_FOLDER_NAME,
    )
    .await?;
    init_folder(
        &serve_path,
        crate::serve::repository::file_system::PLAYLIST_FOLDER_NAME,
    )
    .await?;
    init_folder(
        &serve_path,
        crate::serve::repository::file_system::DEVICE_FOLDER_NAME,
    )
    .await?;
    init_folder(
        &serve_path,
        crate::serve::repository::file_system::FONTS_FOLDER_NAME,
    )
    .await?;

    Ok(())
}

pub async fn init_serve_config(serve_path: &Path) -> anyhow::Result<()> {
    let path = get_serve_config_path(serve_path);

    if path.exists() {
        return Ok(());
    }

    let system_timezone = iana_time_zone::get_timezone()?.parse()?;

    let config = SlipwayServeConfig {
        log_level: Some("info".to_string()),
        registry_urls: vec![
            "file:./components/{publisher}.{name}.{version}.tar".to_string(),
            "file:./components/{publisher}.{name}".to_string(),
        ],
        timezone: Some(system_timezone),
        repository: RepositoryConfig::Filesystem,
        ..SlipwayServeConfig::default()
    };

    info!("Adding config: {path:?}");
    crate::serve::repository::file_system::write_json_to_file(
        &path,
        "Slipway Serve Config",
        &config,
    )
    .await?;

    Ok(())
}

const DOCKERFILE_CONTENT: &str = include_str!("Dockerfile");
pub async fn init_dockerfile(serve_path: &Path) -> anyhow::Result<()> {
    let path = serve_path.join("Dockerfile");

    if path.exists() {
        return Ok(());
    }

    info!("Adding dockerfile: {path:?}");
    crate::serve::repository::file_system::write_str_to_file(
        &path,
        "Dockerfile",
        DOCKERFILE_CONTENT,
    )
    .await?;

    Ok(())
}

pub async fn init_git_ignore(serve_path: &Path) -> anyhow::Result<()> {
    let path = serve_path.join(".gitignore");

    if path.exists() {
        return Ok(());
    }

    info!("Adding .gitignore: {path:?}");
    crate::serve::repository::file_system::write_str_to_file(
        &path,
        ".gitignore",
        indoc::indoc!(
            r#"
            /aot

            .DS_Store
            "#
        ),
    )
    .await?;

    Ok(())
}

pub async fn init_folder(serve_path: &Path, folder_name: &str) -> anyhow::Result<()> {
    let path = serve_path.join(folder_name);

    if path.exists() {
        return Ok(());
    }

    info!("Adding folder: {path:?}");
    tokio::fs::create_dir_all(path).await?;

    Ok(())
}
