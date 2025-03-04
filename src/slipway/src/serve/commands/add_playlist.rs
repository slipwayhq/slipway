use std::path::PathBuf;

use crate::{
    primitives::{PlaylistName, RigName},
    serve::{
        create_repository, load_serve_config,
        repository::{Playlist, PlaylistItem},
    },
};

pub async fn add_playlist(
    serve_path: PathBuf,
    name: PlaylistName,
    rig: Option<RigName>,
) -> anyhow::Result<()> {
    let config = load_serve_config(&serve_path).await?;
    let repository = create_repository(&serve_path, &config.repository);

    let playlist = Playlist {
        items: match rig {
            None => vec![],
            Some(rig) => vec![PlaylistItem {
                span: None,
                days: None,
                refresh_rate_seconds: 3600,
                rig,
            }],
        },
    };

    repository.set_playlist(&name, &playlist).await?;

    Ok(())
}
