use crate::{OpenPackageError, Package};
use std::path::{Path, PathBuf};
use zip::result::ZipError;

#[derive(Clone, Debug)]
pub enum Source {
    DownloadUrl(String),
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Invalid directory '{0}': {1}")]
    InvalidDirectory(PathBuf, String),

    #[error(transparent)]
    ExtractionFailed(#[from] ExtractError),

    #[error(transparent)]
    InvalidPackage(#[from] OpenPackageError),

    #[error(transparent)]
    GetFailed(#[from] reqwest::Error),

    #[error("Invalid URL '{0}'")]
    InvalidUrl(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ExtractError {
    #[error(transparent)]
    UnzipFailed(#[from] ZipError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub async fn fetch_package(source: Source, directory: impl AsRef<Path>) -> Result<Package, FetchError> {
    let package_root_directory = match source {
        Source::DownloadUrl(url) => download_url(&url, directory).await?,
    };

    Ok(Package::open(package_root_directory)?)
}

fn extract_zip(archive_file_path: &Path, destination_directory: &Path) -> Result<PathBuf, ExtractError> {
    let archive_file = std::fs::File::open(archive_file_path)?;
    let mut archive = zip::ZipArchive::new(archive_file)?;
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
            std::fs::create_dir_all(destination_path)?;
        } else {
            let mut out_file = std::fs::File::create(destination_path)?;

            std::io::copy(&mut zip_file, &mut out_file)?;
        }
    }

    Ok(hud_directory.expect("root directory"))
}

fn extract_file_name(url: &str) -> Option<String> {
    url.rfind('/').and_then(|position| {
        if position + 1 >= url.len() {
            return None;
        }

        Some(url[position + 1..].to_string())
    })
}

async fn download_url(url: &str, directory: impl AsRef<Path>) -> Result<PathBuf, FetchError> {
    let directory = directory.as_ref();
    let response = reqwest::get(url).await?;
    let file_name = extract_file_name(url).ok_or(FetchError::InvalidUrl(url.to_string()))?;
    let archive_file_path = directory.join(file_name);
    let content = response.bytes().await?;

    tokio::fs::write(&archive_file_path, content).await?;

    Ok(extract_zip(&archive_file_path, directory)?)
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
    #[test_case("", None)]
    fn test_extract_file_name(input: &str, expected: Option<String>) {
        assert_eq!(expected, extract_file_name(input))
    }

    #[tokio::test]
    async fn test_fetch() {
        let directory = TempDir::new("test_fetch").unwrap();
        let source = Source::DownloadUrl("https://github.com/n0kk/ahud/archive/refs/heads/master.zip".into());
        let package = fetch_package(source, directory.path()).await.unwrap();

        assert_eq!(package.hud_directories.len(), 1);
        assert_eq!(package.hud_directories[0].name, HudName::new("ahud"));
    }
}
