use crate::source::Source;
use crate::{fetch_package, FetchError, HudDirectory, HudName, Install, OpenHudDirectoryError};
use std::path::{Path, PathBuf};
use tempdir::TempDir;

#[derive(thiserror::Error, Debug)]
pub enum InstallError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    FetchPackageFailed(#[from] FetchError),
    #[error("Hud '{0}' not found")]
    HudNotFound(HudName),
    #[error(transparent)]
    FailedToOpenHud(#[from] OpenHudDirectoryError),
    #[error(transparent)]
    FailedToMoveDirectory(#[from] fs_extra::error::Error),
}

pub async fn install(source: Source, name: HudName, huds_directory: PathBuf) -> Install {
    match install_impl(source, name, huds_directory).await {
        Ok(hud_directory) => Install::installed_now(&hud_directory.path),
        Err(error) => Install::failed(error),
    }
}

async fn install_impl(source: Source, name: HudName, huds_directory: PathBuf) -> Result<HudDirectory, InstallError> {
    let directory = TempDir::new(&format!("install_{}", name))?;
    let package = fetch_package(source, directory.path()).await?;
    let source_directory = package
        .find_hud(&name)
        .ok_or(InstallError::HudNotFound(name.clone()))?
        .path
        .clone();
    let source_name = source_directory.file_name().expect("source file name");
    let destination_directory = huds_directory.join(source_name);
    let copy_options = fs_extra::dir::CopyOptions::new().copy_inside(true);

    assert!(source_directory.is_dir());

    fs_extra::dir::move_dir(&source_directory, &destination_directory, &copy_options)?;

    Ok(HudDirectory::new(&destination_directory).expect("scan destination hud directory"))
}

pub async fn uninstall(hud_directory: &Path, huds_directory: PathBuf) -> Result<(), std::io::Error> {
    assert!(hud_directory.starts_with(&huds_directory));

    tokio::fs::remove_dir_all(hud_directory).await
}

#[cfg(test)]
mod tests {
    use super::install;
    use crate::{HudDirectory, HudName, Source};
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_install_zip() {
        let source = Source::DownloadUrl("https://github.com/n0kk/ahud/archive/refs/heads/master.zip".into());
        let directory = TempDir::new("test_install_zip").unwrap();
        let install = install(source, HudName::new("ahud-master"), directory.path().to_path_buf()).await;
        let hud = HudDirectory::new(install.as_installed().unwrap().0).unwrap();

        assert_eq!(HudName::new("ahud-master"), hud.name);
    }

    #[tokio::test]
    async fn test_install_7z() {
        let source = Source::DownloadUrl("https://www.dropbox.com/s/cwwmppnn3nn68av/3HUD.7z?dl=1".into());
        let directory = TempDir::new("test_install_7z").unwrap();
        let install = install(source, HudName::new("3HUD"), directory.path().to_path_buf()).await;
        let hud = HudDirectory::new(install.as_installed().unwrap().0).unwrap();

        assert_eq!(HudName::new("3HUD"), hud.name);
    }
}
