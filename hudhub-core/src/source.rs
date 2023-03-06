use crate::{OpenPackageError, Package};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::source::archives::{ArchiveError, extract_archive};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Source {
    None,
    DownloadUrl(String),
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Invalid directory '{0}': {1}")]
    InvalidDirectory(PathBuf, String),

    #[error(transparent)]
    ExtractionFailed(#[from] ArchiveError),

    #[error(transparent)]
    InvalidPackage(#[from] OpenPackageError),

    #[error(transparent)]
    GetFailed(#[from] reqwest::Error),

    #[error("Invalid URL '{0}'")]
    InvalidUrl(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub async fn fetch_package(source: Source, directory: impl AsRef<Path>) -> Result<Package, FetchError> {
    let package_root_directory = match source {
        Source::None => { panic!("Trying to fetch a package without source") }
        Source::DownloadUrl(url) => {
            let archive_file_path = download_url(&url, &directory).await?;

            extract_archive(&archive_file_path, &directory)?
        }
    };

    Ok(Package::open(package_root_directory)?)
}



mod archives {
    #[derive(thiserror::Error, Debug)]
    pub enum ArchiveError {
        #[error("Unsupported archive type: '{0}'")]
        UnsupportedArchiveType(PathBuf),
        #[error("Reading archive '{0}' failed: '{1}'")]
        ReadFailed(PathBuf, Box<dyn std::error::Error>),
        #[error("Creating directory '{0}' failed: '{1}'")]
        CreateDirectoryFailed(PathBuf, std::io::Error),
        #[error("Writing file '{0}' failed: '{1}'")]
        CreateFileFailed(PathBuf, std::io::Error),
        #[error("Copying file '{0}' failed: '{1}'")]
        CopyFileFailed(PathBuf, std::io::Error),
    }

    use std::path::{Path, PathBuf};

    #[derive(thiserror::Error, Debug)]
    #[error("Failed to unrar archive: {0}")]
    struct RarError(String);

    pub fn extract_archive(archive_file_path: &Path, destination_directory: impl AsRef<Path>) -> Result<PathBuf, ArchiveError> {
        match archive_file_path.extension().and_then(|extension| extension.to_str()) {
            Some("zip") => extract_zip(archive_file_path, destination_directory),
            Some("7z") => extract_7z(archive_file_path, destination_directory),
            Some("rar") => extract_rar(archive_file_path, destination_directory),
            _ => Err(ArchiveError::UnsupportedArchiveType(archive_file_path.to_path_buf())),
        }
    }

    fn extract_zip(archive_file_path: &Path, destination_directory: impl AsRef<Path>) -> Result<PathBuf, ArchiveError> {
        let destination_directory = destination_directory.as_ref();
        let archive_file = std::fs::File::open(archive_file_path).map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(e)))?;
        let mut archive = zip::ZipArchive::new(archive_file)
            .map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(e)))?;
        let mut hud_directory: Option<PathBuf> = None;

        if !archive.is_empty() {
            let zip_file = archive.by_index(0).unwrap();
            assert!(zip_file.is_dir());
            hud_directory = Some(destination_directory.join(zip_file.name()));
        }

        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i).unwrap();
            let zip_file_name = match zip_file.enclosed_name() {
                Some(path) => path,
                None => continue,
            };
            let destination_path = destination_directory.join(zip_file_name);

            if zip_file.name().ends_with("/") {
                std::fs::create_dir_all(&destination_path).map_err(|e| ArchiveError::CreateDirectoryFailed(destination_path.to_path_buf(), e))?;
            } else {
                let mut out_file = std::fs::File::create(&destination_path).map_err(|e| ArchiveError::CreateFileFailed(destination_path.to_path_buf(), e))?;

                std::io::copy(&mut zip_file, &mut out_file).map_err(|e| ArchiveError::CopyFileFailed(destination_path.to_path_buf(), e))?;
            }
        }

        Ok(hud_directory.expect("root directory"))
    }

    fn extract_7z(archive_file_path: &Path, destination_directory: impl AsRef<Path>) -> Result<PathBuf, ArchiveError> {
        let destination_directory = destination_directory.as_ref();

        sevenz_rust::decompress_file(archive_file_path, destination_directory).map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(e)))?;

        Ok(destination_directory.to_path_buf())
    }

    fn extract_rar(archive_file_path: &Path, destination_directory: impl AsRef<Path>) -> Result<PathBuf, ArchiveError> {
        let destination_directory = destination_directory.as_ref();

        unrar::Archive::new(archive_file_path)
            .map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(RarError(e.to_string()))))?
            .extract_to(destination_directory).map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(RarError(e.to_string()))))?
            .process().map_err(|e| ArchiveError::ReadFailed(archive_file_path.to_path_buf(), Box::new(RarError(e.to_string()))))?;


        Ok(destination_directory.to_path_buf())
    }
}

fn get_file_name(url: &str, response: &reqwest::Response) -> Option<String> {
    if let Some(file_name) = extract_file_name(url) {
        if is_valid_filename_with_extension(&file_name) {
            return Some(file_name)
        }
    }

    if let Some(file_name) = extract_file_name(response.url().path()) {
        if is_valid_filename_with_extension(&file_name) {
            return Some(file_name)
        }
    }

    None
}


fn is_valid_filename_with_extension(file_name: &str) -> bool {
    PathBuf::from(file_name).extension().is_some()
}

fn extract_file_name(url: &str) -> Option<String> {
    url.rfind('/').and_then(|position| {
        if position + 1 >= url.len() {
            return None;
        }

        if let Some(end_position) = url[position + 1..].find('?') {
            Some(url[position + 1..end_position + position + 1].to_string())
        }
        else {
            Some(url[position + 1..].to_string())
        }
    })
}

async fn download_url(url: &str, directory: impl AsRef<Path>) -> Result<PathBuf, FetchError> {
    let directory = directory.as_ref();
    let response = reqwest::get(url).await?;
    let file_name = get_file_name(url, &response).ok_or(FetchError::InvalidUrl(url.to_string()))?;
    let archive_file_path = directory.join(file_name);
    let content = response.bytes().await?;

    tokio::fs::write(&archive_file_path, content).await?;

    Ok(archive_file_path)
}

#[cfg(test)]
mod tests {
    use super::{extract_file_name, fetch_package, Package};
    use crate::{HudName, Source};
    use tempdir::TempDir;
    use test_case::test_case;

    #[test_case(
        "https://github.com/n0kk/ahud/archive/refs/heads/master.zip",
        Some(String::from("master.zip"))
    )]
    #[test_case("https://github.com/n0kk/ahud/archive/refs/heads/", None)]
    #[test_case("https://www.dropbox.com/s/cwwmppnn3nn68av/3HUD.7z?dl=1", Some(String::from("3HUD.7z")))]
    #[test_case("", None)]
    fn test_extract_file_name(input: &str, expected: Option<String>) {
        assert_eq!(expected, extract_file_name(input))
    }

    #[tokio::test]
    async fn test_fetch_zip() {
        let directory = TempDir::new("test_fetch_zip").unwrap();
        let source = Source::DownloadUrl("https://github.com/n0kk/ahud/archive/refs/heads/master.zip".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("ahud"));
    }

    #[tokio::test]
    async fn test_fetch_7z() {
        let directory = TempDir::new("test_fetch_7z").unwrap();
        let source = Source::DownloadUrl("https://www.dropbox.com/s/cwwmppnn3nn68av/3HUD.7z?dl=1".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("3hud"));
    }

    #[tokio::test]
    async fn test_fetch_rar() {
        let directory = TempDir::new("test_fetch_rar").unwrap();
        let source = Source::DownloadUrl("https://gamebanana.com/dl/815166".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("HL Hud"));
    }

    #[tokio::test]
    async fn test_fetch_7z_gamebanana() {
        let directory = TempDir::new("test_fetch_7z_gamebanana").unwrap();
        let source = Source::DownloadUrl("https://gamebanana.com/dl/601806".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("7hud"));
    }

    #[tokio::test]
    async fn test_fetch_gamebanana_hud_name_without_quotes() {
        let directory = TempDir::new("test_fetch_gamebanana").unwrap();
        let source = Source::DownloadUrl("https://gamebanana.com/dl/601806".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("7hud"));
    }


}
