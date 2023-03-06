//! Model and functions to manipulate installation packages.
//! Usually, an installation package contains one file info.vdf, but it can contain
//! more than one if the package contains multiple HUDs.

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

const INFO_VDF_FILE_NAME: &str = "info.vdf";

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct HudName(String);

impl HudName {
    pub fn new(name: impl ToString) -> Self {
        Self(name.to_string())
    }
}

impl Display for HudName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// The root directory of a HUD.
#[derive(Clone)]
pub struct HudDirectory {
    /// The path to the directory, relative to the package root directory.
    pub path: PathBuf,
    /// The name parsed in the info.vdf file.
    /// This is the unique identifier of the HUD.
    pub name: HudName,
}

impl HudDirectory {
    pub fn scan(path: impl AsRef<Path>) -> Result<Self, OpenHudDirectoryError> {
        let path = path.as_ref().to_path_buf();
        let info_vdf_file_path = std::fs::read_to_string(path.join(INFO_VDF_FILE_NAME))
            .map_err(|e| OpenHudDirectoryError::FailedToReadVdfFile(e))?;
        let name = Self::parse_name_in_vdf(&info_vdf_file_path).ok_or(OpenHudDirectoryError::FailedToFindHudName)?;

        Ok(Self { path, name })
    }

    fn parse_name_in_vdf(input: &str) -> Option<HudName> {
        const QUOTE: char = '\"';
        let first_quote_pos = input.find(QUOTE)?;
        let second_quote_pos = input[first_quote_pos + 1..].find(QUOTE)? + first_quote_pos;
        let name = input[first_quote_pos + 1..second_quote_pos + 1].to_string();

        Some(HudName(name))
    }
}

/// A package that contains 0 - n [`HudDirectory`].
pub struct Package {
    pub root_directory: PathBuf,
    pub hud_directories: Vec<HudDirectory>,
}

impl Package {
    pub fn open(root_directory: impl Into<PathBuf>) -> Result<Self, OpenPackageError> {
        let root_directory = root_directory.into();

        Ok(Self {
            root_directory: root_directory.clone(),
            hud_directories: Self::scan(&root_directory)?,
        })
    }

    pub fn hud_names(&self) -> impl Iterator<Item = &HudName> {
        self.hud_directories.iter().map(|directory|&directory.name)
    }

    pub fn find_hud(&self, name: &HudName) -> Option<&HudDirectory> {
        self.hud_directories.iter().find(|directory| &directory.name == name)
    }

    fn scan(root_directory: &Path) -> Result<Vec<HudDirectory>, ScanPackageError> {
        let mut hud_directories = Vec::new();

        // Scan root directory
        {
            let info_vdf_file_path = root_directory.join(INFO_VDF_FILE_NAME);

            if info_vdf_file_path.exists() {
                hud_directories.push(HudDirectory::scan(root_directory)?);
            }
        }

        // Scan sub directories
        for entry in std::fs::read_dir(root_directory)
            .map_err(|e| ScanPackageError::CantReadDirectory(root_directory.to_path_buf(), e))?
        {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let info_vdf_file_path = entry.path().join(INFO_VDF_FILE_NAME);

                        if info_vdf_file_path.exists() {
                            hud_directories.push(HudDirectory::scan(entry.path())?);
                        }
                    }
                }
            }
        }

        Ok(hud_directories)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OpenHudDirectoryError {
    #[error("Failed to find HUD's name in info.vdf")]
    FailedToFindHudName,

    #[error("Failed to read .vdf file")]
    FailedToReadVdfFile(std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum OpenPackageError {
    #[error(transparent)]
    ScanFailed(#[from] ScanPackageError),
}

#[derive(thiserror::Error, Debug)]
pub enum ScanPackageError {
    #[error("Can't read directory '{0}': {1}")]
    CantReadDirectory(PathBuf, std::io::Error),
    #[error(transparent)]
    CantOpenHudDirectory(#[from] OpenHudDirectoryError),
}

#[cfg(test)]
mod tests {
    use crate::package::{HudDirectory, HudName, Package};
    use std::path::Path;
    use tempdir::TempDir;
    use test_case::test_case;

    #[test_case("\"Hello\"", Some(HudName(String::from("Hello"))))]
    #[test_case("\n\"Hello\"", Some(HudName(String::from("Hello"))))]
    #[test_case("\"\"", Some(HudName(String::from(""))))]
    #[test_case("\"", None)]
    fn parse_name_in_vdf(input: &str, expected: Option<HudName>) {
        assert_eq!(expected, HudDirectory::parse_name_in_vdf(input));
    }

    fn create_vdf_file(name: &str, directory: &Path) {
        let mut content = format!("\"{}\"\n", name);
        content.push_str("{\n    \"ui_version\"    \"3\"\n}");
        std::fs::write(directory.join("info.vdf"), content).unwrap();
    }

    #[test]
    fn test_open_package_one_vdf() {
        let package_dir = TempDir::new("test_open_package_one_vdf").unwrap();
        create_vdf_file("test", package_dir.path());

        let package = Package::open(package_dir.path()).unwrap();

        assert_eq!(1, package.hud_directories.len());
        assert_eq!(HudName("test".into()), package.hud_directories[0].name);
    }

    #[test]
    fn test_open_package_multiple_vdf() {
        let package_dir = TempDir::new("test_open_package_one_vdf").unwrap();
        let d0 = package_dir.path().join("d0");
        let d1 = package_dir.path().join("d1");
        std::fs::create_dir_all(&d0).unwrap();
        std::fs::create_dir_all(&d1).unwrap();
        create_vdf_file("test0", &d0);
        create_vdf_file("test1", &d1);

        let package = Package::open(package_dir.path()).unwrap();

        assert_eq!(2, package.hud_directories.len());
        assert_eq!(HudName("test0".into()), package.hud_directories[0].name);
        assert_eq!(HudName("test1".into()), package.hud_directories[1].name);
    }
}
