use hudhub_core::Registry;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct State {
    pub registry: Registry,
}

#[derive(thiserror::Error, Debug)]
pub enum LoadStateError {
    #[error(transparent)]
    IoError(#[from]std::io::Error),
    #[error("Invalid file format")]
    InvalidFileFormat,
}

impl State {
    pub async fn save(state: &State, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent_path) = path.parent() {
            if !parent_path.exists() {
                tokio::fs::create_dir_all(parent_path).await?;
            }
        }

        let encoded: Vec<u8> = bincode::serialize(&state).expect("serialize state");

        tokio::fs::write(path, encoded).await
    }

    pub async fn load(path: &Path) -> Result<State, LoadStateError> {
        match path.exists() {
            true => {
                let encoded = tokio::fs::read(path).await?;

                Ok(bincode::deserialize(&encoded).map_err(|_| LoadStateError::InvalidFileFormat)?)
            }
            false => Ok(State::default()),
        }
    }
}
