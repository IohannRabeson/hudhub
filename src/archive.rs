use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use zip::result::ZipError;

#[derive(thiserror::Error, Debug)]
pub enum ExtractError {
    #[error("Unsupported archive type: '{0}'")]
    UnsupportedArchiveType(PathBuf),
    #[error(transparent)]
    UnzipFailed(#[from] ZipError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("No file info.vdf found")]
    NoInfoFileFound,
}

pub fn extract(archive: &Path, destination_directory: &Path) -> Result<PathBuf, ExtractError> {
    match archive.extension().map(|extension| extension.as_bytes()) {
        Some(b"zip") => extract_zip(archive, destination_directory),
        _ => Err(ExtractError::UnsupportedArchiveType(archive.to_path_buf())),
    }
}

fn extract_zip(
    archive_file_path: &Path,
    destination_directory: &Path,
) -> Result<PathBuf, ExtractError> {
    let archive_file = std::fs::File::open(archive_file_path)?;
    let mut archive = zip::ZipArchive::new(archive_file)?;
    let mut hud_directory: Option<PathBuf> = None;

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
            if zip_file_name.ends_with("info.vdf") {
                assert!(hud_directory.is_none());
                hud_directory = destination_path.parent().map(|path| path.to_path_buf());
            }
            let mut out_file = std::fs::File::create(destination_path)?;

            std::io::copy(&mut zip_file, &mut out_file)?;
        }
    }

    hud_directory.ok_or(ExtractError::NoInfoFileFound)
}
