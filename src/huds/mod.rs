use crate::huds::hud::Hud;
use crate::huds::archive_location::FetchError;
pub use archive_location::ArchiveLocation;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::archive::ExtractError;

pub mod hud;
pub mod archive_location;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Huds {
    huds: BTreeMap<ArchiveLocation, HudInfo>,
}

impl Huds {
    pub fn new() -> Self {
        Self {
            huds: BTreeMap::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &HudInfo> {
        self.huds.values()
    }

    pub fn add(&mut self, hud: Hud) -> ArchiveLocation {
        let key = hud.archive_location.clone();

        self.huds.insert(key.clone(), hud.into());

        key
    }

    pub fn get(&self, source: &ArchiveLocation) -> Option<&HudInfo> {
        self.huds.get(source)
    }

    pub fn set_install(&mut self, key: &ArchiveLocation, status: Install) {
        if let Some(info) = self.huds.get_mut(key) {
            info.install = status;
        }
    }

    // pub async fn install(
    //     key: ArchiveLocation,
    //     huds_directory: PathBuf,
    // ) -> Result<(), InstallError> {
    //
    //     Self::install_hud(&key, &huds_directory).await
    // }
    //
    // pub async fn uninstall(
    //     &mut self,
    //     key: &ArchiveLocation,
    //     huds_directory: &Path,
    // ) -> Result<(), UninstallError> {
    //     match self.huds.get_mut(key) {
    //         Some(hud_info) => {
    //             if let Install::Installed { path, .. } = &hud_info.install {
    //                 Self::uninstall_hud(path, huds_directory).await?;
    //                 hud_info.install = Install::None;
    //             }
    //         }
    //         None => {
    //             panic!("No HUB registered to '{:?}'", key);
    //         }
    //     }
    //
    //     Ok(())
    // }
    //
    //
    // async fn install_hud(source: &ArchiveLocation, huds_directory: &Path) -> Result<PathBuf, InstallError> {
    //     tokio::fs::create_dir_all(&huds_directory)
    //         .await
    //         .map_err(|e| InstallError::FailedToCreateDirectory(e, huds_directory.to_path_buf()))?;
    //
    //     let temp_directory = tempdir::TempDir::new("tf2-huds-manager-temporary")
    //         .map_err(|e| InstallError::FailedToCreateDirectory(e, huds_directory.to_path_buf()))?;
    //     let archive_path = archive_location::fetch_archive(source, temp_directory.path()).await?;
    //
    //     archive::extract(&archive_path, &huds_directory).map_err(InstallError::FailedToExtractHud)
    // }
    //
    // async fn uninstall_hud(
    //     install_path: &Path,
    //     huds_directory: &Path,
    // ) -> Result<(), UninstallError> {
    //     assert!(install_path.starts_with(huds_directory));
    //
    //     tokio::fs::remove_dir_all(install_path)
    //         .await
    //         .map_err(|e| UninstallError::FailedToRemoveDirectory(e, install_path.to_path_buf()))?;
    //
    //     Ok(())
    // }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HudInfo {
    pub hud: Hud,
    pub install: Install,
    pub added: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Install {
    None,
    Installed { path: PathBuf, timestamp: DateTime<Utc> },
    Failed { error: String },
}

impl Install {
    pub fn is_installed(&self) -> bool {
        matches!(self, Install::Installed { .. })
    }
}

impl From<Hud> for HudInfo {
    fn from(hud: Hud) -> Self {
        Self {
            hud,
            install: Install::None,
            added: Utc::now(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InstallError {
    #[error("Failed to create the directory '{1}': {0}")]
    FailedToCreateDirectory(std::io::Error, PathBuf),
    #[error("Failed to fetch a hud: {0}")]
    FailedToFetchHud(#[from] FetchError),
    #[error("Failed to extract a hud: {0}")]
    FailedToExtractHud(#[from] ExtractError),
}

#[derive(thiserror::Error, Debug)]
pub enum UninstallError {
    #[error("Failed to remove directory '{0}': {1}")]
    FailedToRemoveDirectory(std::io::Error, PathBuf),
}

#[cfg(test)]
mod tests {
    use crate::huds::hud::Hud;
    use crate::huds::archive_location::ArchiveLocation;
    use crate::huds::{Huds, Install};
    use reqwest::Url;
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_download_install_uninstall() {
        let huds_directory = TempDir::new("test_install").unwrap();
        let mut manager = Huds::new();

        let hud_key = manager.add(Hud {
            archive_location: ArchiveLocation::DownloadUrl(String::from(
                "https://github.com/n0kk/ahud/archive/refs/heads/master.zip",
            )),
            display_name: "ahud".to_string(),
        });
        manager
            .install(&hud_key, &huds_directory.path())
            .await
            .unwrap();
        assert!(
            matches!(manager.get(&hud_key).map(|info|info.install.clone()), Some(Install::Installed { path, .. }) if path.starts_with(huds_directory.path()))
        );
        if let Some(Install::Installed { path, timestamp }) = manager.get(&hud_key).map(|info|&info.install) {
            assert!(path.join("info.vdf").exists());
        }
        manager
            .uninstall(&hud_key, huds_directory.path())
            .await
            .unwrap();
    }
}
