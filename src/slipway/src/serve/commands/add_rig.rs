use std::path::PathBuf;

use crate::{
    CommonPermissionsArgs,
    primitives::RigName,
    serve::{create_repository, load_serve_config, save_serve_config},
};

pub async fn add_rig(
    serve_path: PathBuf,
    name: RigName,
    permissions: CommonPermissionsArgs,
) -> anyhow::Result<()> {
    let mut config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let rig = slipway_engine::Rig {
        description: None,
        constants: None,
        rigging: slipway_engine::Rigging {
            components: Default::default(),
        },
        context: None,
    };

    repository.set_rig(&name, &rig).await?;

    let new_permissions = permissions.into_permissions()?;

    config.rig_permissions.insert(name, new_permissions);

    save_serve_config(&serve_path, &config).await?;

    Ok(())
}
