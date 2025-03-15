use std::{collections::HashMap, path::PathBuf};

use slipway_engine::{BasicComponentCache, BasicComponentsLoader};
use tracing::info;

use crate::serve::{create_repository, load_serve_config};

pub async fn consolidate(serve_path: PathBuf) -> anyhow::Result<BasicComponentCache> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    info!("Consolidating. Any remote components will be cached locally.");

    let rigs = repository.list_rigs().await?;

    let components_loader = BasicComponentsLoader::builder()
        .local_base_directory(&serve_path)
        .registry_lookup_urls(config.registry_urls.clone())
        .build();

    let mut all_components = HashMap::new();
    for rig in rigs {
        info!("Consolidating rig: {rig}");
        let rig = repository.get_rig(&rig).await?;
        let cache = BasicComponentCache::primed(&rig, &components_loader).await?;
        let components = cache.into_inner();
        all_components.extend(components);
    }

    info!("Done.");

    let all_components_cache = BasicComponentCache::for_primed(all_components);
    Ok(all_components_cache)
}
