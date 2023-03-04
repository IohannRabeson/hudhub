use reqwest::{IntoUrl, Url};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ArchiveLocation {
    DownloadUrl(String),
    AbsolutePath(PathBuf),
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("Failed to download archive: {0}")]
    DownloadFailed(#[from] reqwest::Error),
    #[error("Failed to write archive: {0}")]
    WriteFailed(#[from] std::io::Error),
    #[error("The URL '{0}' is invalid")]
    InvalidUrl(String),
}

pub async fn fetch_archive(
    source: &ArchiveLocation,
    temporary_directory: &Path,
) -> Result<PathBuf, FetchError> {
    match source {
        ArchiveLocation::DownloadUrl(url) => download_archive(url, temporary_directory).await,
        ArchiveLocation::AbsolutePath(path) => Ok(path.clone()),
    }
}

async fn download_archive(url: &str, temporary_directory: &Path) -> Result<PathBuf, FetchError> {
    let response = reqwest::get(url).await?;
    let file_name = extract_file_name(url).ok_or(FetchError::InvalidUrl(url.to_string()))?;
    let destination_file_path = temporary_directory.join(file_name);
    let content = response.bytes().await?;

    tokio::fs::write(&destination_file_path, content).await?;

    Ok(destination_file_path)
}

fn extract_file_name(url: &str) -> Option<String> {
    url.rfind('/').and_then(|position| {
        if position + 1 >= url.len() {
            return None;
        }

        Some(url[position + 1..].to_string())
    })
}

#[cfg(test)]
mod tests {
    use crate::huds::archive_location::extract_file_name;
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
}
