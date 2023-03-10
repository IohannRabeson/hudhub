use std::cell::RefCell;
use std::path::PathBuf;
use platform_dirs::AppDirs;
use steamlocate::SteamDir;
use tempdir::TempDir;

/// Provides all the paths needed by this application.
pub trait PathsProvider {
    fn get_application_directory(&self) -> PathBuf;
    fn get_team_fortress_directory(&self) -> Option<PathBuf>;

    fn get_application_state_file_path(&self) -> PathBuf {
        self.get_application_directory().join("application.state")
    }

    fn get_huds_directory(&self) -> Option<PathBuf> {
        self.get_team_fortress_directory().map(|directory| directory.join("tf").join("custom"))
    }
}

/// Provides the paths found on the disk.
pub struct DefaultPathsProvider {
    application_directories: AppDirs,
    steam_directory: RefCell<SteamDir>,
}

impl DefaultPathsProvider {
    pub fn new() -> Self {
        Self {
            application_directories: AppDirs::new(Some("hudhub"), true).expect("create AppDirs"),
            steam_directory: RefCell::new(SteamDir::locate().expect("create SteamDir")),
        }
    }
}

impl PathsProvider for DefaultPathsProvider {
    fn get_application_directory(&self) -> PathBuf {
        self.application_directories.config_dir.clone()
    }

    fn get_team_fortress_directory(&self) -> Option<PathBuf> {
        const TEAM_FORTRESS_2_STEAM_APP_ID: u32 = 440;

        return self.steam_directory.borrow_mut().app(&TEAM_FORTRESS_2_STEAM_APP_ID).map(|dir| dir.path.clone());
    }
}

/// Provides paths located in a temporary directory deleted when the application quits.
pub struct TestPathsProvider {
    temporary_directory: TempDir,
}

impl TestPathsProvider {
    pub fn new() -> Self {
        Self {
            temporary_directory: TempDir::new("test_paths_provider").expect("create temporary directory for test"),
        }
    }
}

impl PathsProvider for TestPathsProvider {
    fn get_application_directory(&self) -> PathBuf {
        self.temporary_directory.path().join("application_directory")
    }

    fn get_team_fortress_directory(&self) -> Option<PathBuf> {
        Some(self.temporary_directory.path().join("team_fortress_directory"))
    }
}