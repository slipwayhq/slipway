use std::path::PathBuf;

use slipway_engine::Publisher;

use crate::{
    primitives::RigName,
    serve::{create_repository, load_serve_config},
};

pub async fn add_rig(
    serve_path: PathBuf,
    name: RigName,
    publisher: Publisher,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let rig = slipway_engine::Rig {
        name,
        publisher,
        version: "1.0.0".to_string(),
        description: "".to_string(),
        constants: None,
        rigging: slipway_engine::Rigging {
            components: Default::default(),
        },
    };

    repository.set_rig(&name, &rig).await?;

    Ok(())
}
