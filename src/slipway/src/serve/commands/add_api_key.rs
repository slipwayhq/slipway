use std::path::PathBuf;

use slipway_host::hash_string;
use tracing::{info, warn};

use crate::{
    primitives::ApiKeyName,
    serve::{create_api_key, load_serve_config, save_serve_config, write_redeploy_warning},
};

pub async fn add_api_key(
    serve_path: PathBuf,
    name: ApiKeyName,
    api_key: Option<String>,
) -> anyhow::Result<()> {
    let mut config = load_serve_config(&serve_path).await?;

    let api_key = api_key.unwrap_or_else(create_api_key);
    let hashed_api_key = hash_string(&api_key);

    info!(
        "Adding API key \"{}\" with unhashed value \"{}\" to Slipway Serve config.",
        name, api_key
    );

    warn!(
        "The unhashed API key is not stored by the server. If you need a record of it, store it securely."
    );

    config.hashed_api_keys.insert(name, hashed_api_key);

    save_serve_config(&serve_path, &config).await?;
    write_redeploy_warning();

    Ok(())
}
