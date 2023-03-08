use crate::source::Source;
use crate::{fetch_package, FetchError, PackageEntry, HudName, Install, OpenHudDirectoryError};
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

async fn install_impl(source: Source, name: HudName, huds_directory: PathBuf) -> Result<PackageEntry, InstallError> {
    let directory = TempDir::new(&format!("install_{}", name))?;
    let package = fetch_package(source, directory.path()).await?;
    let source_hud_path = package
        .find_hud(&name)
        .ok_or(InstallError::HudNotFound(name.clone()))?
        .path
        .clone();
    let source_name = source_hud_path.file_name().expect("source file name");
    let destination_path = huds_directory.join(source_name);

    if source_hud_path.is_dir() {
        let copy_options = fs_extra::dir::CopyOptions::new().copy_inside(true);

        fs_extra::dir::move_dir(&source_hud_path, &destination_path, &copy_options)?;

        return Ok(PackageEntry::directory(&destination_path).expect("scan destination hud directory"))
    } else if source_hud_path.is_file() {
        let copy_options = fs_extra::file::CopyOptions::new().overwrite(true);

        fs_extra::file::copy(&source_hud_path, &destination_path, &copy_options)?;

        return Ok(PackageEntry::vpk_file(&destination_path).expect("scan vpk hud"))
    }
    panic!("Unsupported HUD type");
}

pub async fn uninstall(hud_path: &Path, huds_directory: PathBuf) -> Result<(), std::io::Error> {
    assert!(hud_path.starts_with(&huds_directory));

    if hud_path.is_dir() {
        return tokio::fs::remove_dir_all(hud_path).await
    }

    if hud_path.is_file() {
        return tokio::fs::remove_file(hud_path).await
    }

    panic!("Unsupported HUD type");
}

#[cfg(test)]
mod slow_tests {
    use super::install;
    use crate::{PackageEntry, HudName, Source};
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_install_zip() {
        let source = Source::DownloadUrl("https://github.com/n0kk/ahud/archive/refs/heads/master.zip".into());
        let directory = TempDir::new("test_install_zip").unwrap();
        let install = install(source, HudName::new("ahud-master"), directory.path().to_path_buf()).await;
        let hud = PackageEntry::directory(install.as_installed().unwrap().0).unwrap();

        assert_eq!(HudName::new("ahud-master"), hud.name);
    }

    #[tokio::test]
    async fn test_install_7z() {
        let source = Source::DownloadUrl("https://www.dropbox.com/s/cwwmppnn3nn68av/3HUD.7z?dl=1".into());
        let directory = TempDir::new("test_install_7z").unwrap();
        let install = install(source, HudName::new("3HUD"), directory.path().to_path_buf()).await;
        let hud = PackageEntry::directory(install.as_installed().unwrap().0).unwrap();

        assert_eq!(HudName::new("3HUD"), hud.name);
    }

    #[tokio::test]
    async fn test_install_vpk() {
        let source = Source::DownloadUrl("https://gamebanana.com/dl/945012".into());
        let directory = TempDir::new("test_install_vpk").unwrap();
        let install = install(source, HudName::new("minhud_plus"), directory.path().to_path_buf()).await;
        let hud = PackageEntry::vpk_file(install.as_installed().unwrap().0).unwrap();

        assert_eq!(HudName::new("minhud_plus"), hud.name);
        assert!(directory.path().join("minhud_plus.vpk").exists());
    }
}
