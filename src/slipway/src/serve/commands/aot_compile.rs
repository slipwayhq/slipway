use std::{path::PathBuf, sync::Arc};

use slipway_engine::BasicComponentCache;
use tracing::info;

use crate::component_runners::get_component_runners;

pub async fn aot_compile(
    aot_path: PathBuf,
    component_cache: BasicComponentCache,
) -> anyhow::Result<()> {
    let component_runners = get_component_runners();

    let components = component_cache.into_inner();

    tokio::fs::create_dir_all(&aot_path).await?;

    for (name, component) in components {
        for runner in component_runners.iter() {
            match runner
                .aot_compile(&name, &aot_path, Arc::clone(&component.files))
                .await?
            {
                slipway_engine::TryAotCompileComponentResult::Compiled => {
                    info!("AOT compiled \"{name}\" with \"{}\".", runner.identifier());
                }
                slipway_engine::TryAotCompileComponentResult::CannotCompile => {}
            }
        }
    }

    Ok(())
}
