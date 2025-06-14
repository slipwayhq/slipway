use std::collections::HashMap;
use std::path::{Path, PathBuf};

use actix_cors::Cors;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::{NormalizePath, TrailingSlash, from_fn};
use actix_web::{App, HttpMessage, HttpRequest, HttpServer, web};
use anyhow::Context;
use chrono_tz::Tz;
use repository::{Device, Playlist, ServeRepository};
use serde::{Deserialize, Serialize};

use slipway_engine::TEST_TIMEZONE;
use tracing::{debug, info, warn};

use crate::permissions::PermissionsOwned;
use crate::primitives::{DeviceName, PlaylistName, RigName};
use crate::serve::responses::ServeError;

#[cfg(test)]
mod api_tests;
mod auth;
mod bmp;
pub(super) mod commands;
mod devices;
mod favicon;
mod playlists;
mod repository;
mod responses;
mod rigs;
pub(super) mod trmnl;

const SLIPWAY_SECRET_KEY: &str = "SLIPWAY_SECRET";

const REFRESH_RATE_HEADER: &str = "refresh-rate";
const ACCESS_TOKEN_HEADER: &str = "access-token";
const AUTHORIZATION_HEADER: &str = "authorization";
const ID_HEADER: &str = "id";

const TRMNL_PATH: &str = "/trmnl/api";
const TRMNL_DISPLAY_PATH: &str = "/trmnl/api/display";
const API_GET_RIG_PATH: &str = "/rigs";
const API_GET_DEVICE_PATH: &str = "/devices";

const SERVE_CONFIG_FILE_NAME: &str = "slipway_serve.json";

const GENERATED_API_KEY_LENGTH: usize = 52;

fn truncate_hashed_api_key(hashed_api_key: &str) -> &str {
    &hashed_api_key[..6]
}

pub(super) fn create_api_key() -> String {
    nanoid::nanoid!(GENERATED_API_KEY_LENGTH)
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

    #[serde(flatten)]
    environment: SlipwayServeEnvironment,

    #[serde(default)]
    rig_permissions: HashMap<RigName, PermissionsOwned>,

    #[serde(default)]
    api_keys: Vec<RegisteredApiKey>,

    #[serde(default, skip_serializing_if = "ShowApiKeys::is_default")]
    show_api_keys: ShowApiKeys,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,

    #[serde(default, skip_serializing_if = "RepositoryConfig::is_default")]
    repository: RepositoryConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct SlipwayServeEnvironment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timezone: Option<Tz>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    locale: Option<String>,
}

impl SlipwayServeEnvironment {
    pub fn for_test() -> Self {
        SlipwayServeEnvironment {
            timezone: Some(Tz::Canada__Eastern),
            locale: Some(TEST_TIMEZONE.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct RegisteredApiKey {
    hashed_key: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<DeviceName>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
enum ShowApiKeys {
    #[default]
    Never,
    New,
    Always,
}

impl ShowApiKeys {
    pub fn is_default(&self) -> bool {
        matches!(self, ShowApiKeys::Never)
    }
}

#[derive(Clone)]
struct RequestState {
    pub supplied_api_key: Option<SuppliedApiKey>,
}

#[derive(Clone)]
struct SuppliedApiKey {
    pub api_key: String,
    pub resolved: Option<RegisteredApiKey>,
}

#[derive(Clone)]
struct SuppliedResolvedApiKey {
    pub api_key: String,
    pub resolved: RegisteredApiKey,
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

fn write_api_key_message(api_key: &str) {
    debug!("The API key sent by the device was: {}", api_key);
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
    let port = config.port.unwrap_or(8080);

    HttpServer::new(move || {
        create_app(
            root.clone(),
            aot_path.clone(),
            config.clone(),
            secret.clone(),
        )
    })
    .bind(("0.0.0.0", port))?
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
        .service(favicon::get_favicon)
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

fn get_api_key_from_state(req: &HttpRequest) -> Result<SuppliedApiKey, ServeError> {
    try_get_api_key_from_state(req)
        .ok_or(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            format!("Missing authorization. Please supply an {ACCESS_TOKEN_HEADER} header or an {AUTHORIZATION_HEADER} header or query string parameter."),
        ))
}

fn get_resolved_api_key_from_state(
    req: &HttpRequest,
) -> Result<SuppliedResolvedApiKey, ServeError> {
    let supplied_api_key = get_api_key_from_state(req)?;

    let Some(resolved) = supplied_api_key.resolved else {
        return Err(ServeError::UserFacing(
            StatusCode::UNAUTHORIZED,
            "API key was not recognized.".to_string(),
        ));
    };

    Ok(SuppliedResolvedApiKey {
        api_key: supplied_api_key.api_key,
        resolved,
    })
}

fn try_get_api_key_from_state(req: &HttpRequest) -> Option<SuppliedApiKey> {
    req.extensions()
        .get::<RequestState>()
        .and_then(|state| state.supplied_api_key.clone())
}
