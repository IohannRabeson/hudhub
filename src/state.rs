use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::huds::Huds;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    /// The list of HUDs added.
    pub huds: Huds,
    /// The "custom" folder in the Team Fortress directory.
    pub custom_directory: Option<PathBuf>,
}

impl State {
    pub fn new() -> Self {
        Self {
            huds: Huds::new(),
            custom_directory: None,
        }
    }

    pub async fn save(state: &State, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent_path) = path.parent() {
            if !parent_path.exists() {
                tokio::fs::create_dir_all(parent_path).await?;
            }
        }

        let encoded: Vec<u8> = bincode::serialize(&state).expect("serialize state");

        tokio::fs::write(path, encoded).await
    }

    pub async fn load(path: &Path) -> Result<State, std::io::Error> {
        let encoded = tokio::fs::read(path).await?;

        Ok(bincode::deserialize(&encoded).expect("deserialize state"))
    }
}
