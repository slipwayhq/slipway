use std::collections::HashMap;
use std::path::{Path, PathBuf};

use actix_cors::Cors;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::middleware::{NormalizePath, TrailingSlash, from_fn};
use actix_web::{App, HttpServer, web};
use anyhow::Context;
use chrono_tz::Tz;
use repository::{Device, Playlist, ServeRepository};
use serde::{Deserialize, Serialize};

use tracing::{info, warn};

use crate::permissions::PermissionsOwned;
use crate::primitives::{ApiKeyName, DeviceName, PlaylistName, RigName};

#[cfg(test)]
mod api_tests;
mod auth;
mod bmp;
pub(super) mod commands;
mod devices;
mod playlists;
mod repository;
mod responses;
mod rigs;
pub(super) mod trmnl;

const SLIPWAY_SECRET_KEY: &str = "SLIPWAY_SECRET";

const REFRESH_RATE_HEADER: &str = "refresh-rate";
const ACCESS_TOKEN_HEADER: &str = "access-token";
const ID_HEADER: &str = "id";

const TRMNL_PATH: &str = "/api";
const TRMNL_DISPLAY_PATH: &str = "/api/display";
const API_GET_RIG_PATH: &str = "/rigs";

const SERVE_CONFIG_FILE_NAME: &str = "slipway_serve.json";

fn create_friendly_id(hashed_api_key: &str) -> String {
    hashed_api_key[..6].to_string()
}

fn create_api_key() -> String {
    nanoid::nanoid!(64)
}

#[derive(Debug)]
struct ServeState {
    pub base_path: PathBuf,
    pub aot_path: Option<PathBuf>,
    pub config: SlipwayServeConfig,
    pub secret: Option<String>,
    pub repository: Box<dyn ServeRepository>,
}

impl ServeState {
    pub fn new(
        base_path: PathBuf,
        aot_path: Option<PathBuf>,
        config: SlipwayServeConfig,
        secret: Option<String>,
        repository: Box<dyn ServeRepository>,
    ) -> Self {
        Self {
            base_path,
            aot_path,
            config,
            secret,
            repository,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct SlipwayServeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,

    #[serde(default)]
    registry_urls: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    timezone: Option<Tz>,

    #[serde(default)]
    rig_permissions: HashMap<RigName, PermissionsOwned>,

    #[serde(default)]
    hashed_api_keys: HashMap<ApiKeyName, String>,

    #[serde(default, skip_serializing_if = "RepositoryConfig::is_default")]
    repository: RepositoryConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
enum RepositoryConfig {
    #[default]
    Filesystem,
    Memory {
        devices: HashMap<DeviceName, Device>,
        playlists: HashMap<PlaylistName, Playlist>,
        rigs: HashMap<RigName, slipway_engine::Rig>,
    },
}

impl RepositoryConfig {
    pub fn is_default(&self) -> bool {
        matches!(self, RepositoryConfig::Filesystem)
    }
}

pub async fn serve(path: PathBuf, aot_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config = load_serve_config(&path).await?;
    serve_with_config(path, aot_path, config).await?;
    Ok(())
}

async fn load_serve_config(root_path: &Path) -> Result<SlipwayServeConfig, anyhow::Error> {
    let config_path = root_path.join(SERVE_CONFIG_FILE_NAME);
    let config = match tokio::fs::read(&config_path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).context("Failed to parse Slipway Serve config file.")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SlipwayServeConfig::default(),
        Err(e) => return Err(e).context("Failed to load Slipway Serve config file.")?,
    };
    Ok(config)
}

async fn save_serve_config(
    root_path: &Path,
    config: &SlipwayServeConfig,
) -> Result<(), anyhow::Error> {
    let config_path = root_path.join(SERVE_CONFIG_FILE_NAME);
    let config_bytes =
        serde_json::to_vec_pretty(config).context("Failed to serialize Slipway Serve config")?;
    tokio::fs::write(&config_path, config_bytes)
        .await
        .context("Failed to write Slipway Serve config file")?;
    info!("Saved Slipway Serve config to {}", config_path.display());
    Ok(())
}

fn write_redeploy_warning() {
    warn!("Don't forget to re-deploy if necessary.");
}

fn get_serve_config_path(root_path: &Path) -> PathBuf {
    root_path.join(SERVE_CONFIG_FILE_NAME)
}

fn create_repository(root_path: &Path, config: &RepositoryConfig) -> Box<dyn ServeRepository> {
    match config {
        RepositoryConfig::Filesystem => Box::new(
            repository::file_system::FileSystemRepository::new(root_path.to_owned()),
        ),
        RepositoryConfig::Memory {
            devices,
            playlists,
            rigs,
        } => Box::new(repository::memory::MemoryRepository::new(
            devices.clone(),
            playlists.clone(),
            rigs.clone(),
        )),
    }
}

async fn serve_with_config(
    root: PathBuf,
    aot_path: Option<PathBuf>,
    config: SlipwayServeConfig,
) -> anyhow::Result<()> {
    super::configure_tracing(config.log_level.clone());

    info!("Starting Slipway Serve with config: {:?}", config);

    let secret = std::env::var(SLIPWAY_SECRET_KEY).ok();

    HttpServer::new(move || {
        create_app(
            root.clone(),
            aot_path.clone(),
            config.clone(),
            secret.clone(),
        )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

fn create_app(
    root: PathBuf,
    aot_path: Option<PathBuf>,
    config: SlipwayServeConfig,
    secret: Option<String>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let repository = create_repository(&root, &config.repository);

    App::new()
        .app_data(web::Data::new(ServeState::new(
            root, aot_path, config, secret, repository,
        )))
        .wrap(
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header(),
        )
        .service(
            // Trmnl services.
            web::scope(TRMNL_PATH)
                .wrap(NormalizePath::new(TrailingSlash::Trim)) // Required for TRMNL device as of 2025-03-07.
                .wrap(from_fn(auth::trmnl_auth_middleware))
                .service(trmnl::trmnl_setup)
                .service(trmnl::trmnl_display)
                .service(trmnl::trmnl_log),
        )
        .service(
            // Non-Trmnl services.
            web::scope("")
                .wrap(from_fn(auth::auth_middleware))
                .service(rigs::get_rig::get_rig)
                .service(playlists::get_playlist::get_playlist)
                .service(devices::get_device::get_device),
        )
}

#[derive(Clone)]
struct RequestState {}
