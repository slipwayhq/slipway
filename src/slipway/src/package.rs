use std::{fs::File, io::BufReader, path::Path};

use tar::Builder;
use tracing::{error, info, warn};
use walkdir::WalkDir;

use crate::SLIPWAY_COMPONENT_FILE_NAME;

pub fn package_component(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        error!("Path does not exist: {:?}", path);
        return Ok(());
    }

    if !path.is_dir() {
        error!("Path is not a directory: {:?}", path);
        return Ok(());
    }

    let file_path = path.join(SLIPWAY_COMPONENT_FILE_NAME);

    if !file_path.exists() {
        error!("Component file does not exist: {:?}", file_path);
        return Ok(());
    }

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)?;

    let publisher = json["publisher"].as_str().unwrap_or_default();
    let name = json["name"].as_str().unwrap_or_default();
    let version = json["version"].as_str().unwrap_or_default();

    let tar_name = format!("{}.{}.{}.tar", publisher, name, version);

    let tar_path = match path.parent() {
        Some(parent) => parent.join(tar_name),
        None => {
            warn!(
                "Failed to get component parent folder for, so writing .tar file to component folder."
            );
            path.join(tar_name)
        }
    };

    let tar_file = File::create(&tar_path)?;
    let mut tar_builder = Builder::new(tar_file);

    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(path)?;
            tar_builder.append_path_with_name(entry.path(), rel_path)?;
        }
    }

    tar_builder.finish()?;

    info!("Written component tar file to: {}", tar_path.display());

    Ok(())
}
