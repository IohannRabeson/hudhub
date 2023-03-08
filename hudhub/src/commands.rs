use crate::state::{LoadStateError, State};
use crate::Message;
use hudhub_core::{fetch_package, install, uninstall, FetchError, PackageEntry, HudInfo, HudName, Source, Install};
use iced::Command;
use std::path::{Path, PathBuf};
use tempdir::TempDir;

#[derive(thiserror::Error, Debug)]
enum ScanPackageError {
    #[error(transparent)]
    FetchPackageFailed(#[from] FetchError),

    #[error("Failed to create a temporary directory: {0}")]
    FailedToCreateTempDirectory(std::io::Error),
}

pub fn scan_package(source: Source) -> Command<Message> {
    let source_for_future = source.clone();

    Command::perform(
        async move { get_hud_names(source_for_future).await },
        move |result| match result {
            Err(error) => Message::error("Failed to scan package", error),
            Ok(hud_names) => Message::AddHuds(source, hud_names),
        },
    )
}

async fn get_hud_names(source: Source) -> Result<Vec<HudName>, ScanPackageError> {
    let temp_directory = TempDir::new("fetch_package_name").map_err(ScanPackageError::FailedToCreateTempDirectory)?;
    let package = fetch_package(source.clone(), temp_directory.path()).await?;

    Ok(package.hud_names().cloned().collect())
}

pub fn save_state(state: State, path: impl Into<PathBuf>) -> Command<Message> {
    let path = path.into();

    println!("Save state: {}", path.display());

    Command::perform(async move { State::save(&state, &path).await }, |result| match result {
        Ok(()) => Message::StateSaved,
        Err(error) => Message::error("Failed to save application state", error),
    })
}

fn search_hud_install(huds_directory: &Path) -> Vec<PackageEntry> {
    let mut directories = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(huds_directory) {
        for entry in read_dir {
            if let Ok(entry) = entry {
                if let Ok(entry) = PackageEntry::from_path(entry.path()) {
                    directories.push(entry);
                }
            }
        }
    }

    directories
}

pub fn scan_huds_directory(huds_directory: Option<PathBuf>) -> Command<Message> {
    match huds_directory {
        Some(huds_directory) => Command::perform(
            async move { search_hud_install(&huds_directory) },
            Message::FoundInstalledHuds,
        ),
        None => Command::none(),
    }
}

pub fn load_state(path: impl Into<PathBuf>) -> Command<Message> {
    let path = path.into();

    println!("Load state: {}", path.display());

    Command::perform(
        async move { State::load(&path).await },
        |result: Result<State, LoadStateError>| match result {
            Ok(state) => Message::StateLoaded(state),
            Err(error) => Message::error("Failed to load application state", error),
        },
    )
}

pub fn install_hud(source: Source, name: HudName, huds_directory: PathBuf) -> Command<Message> {
    let hud_name = name.clone();

    Command::perform(
        async move { install(source, hud_name, huds_directory).await },
        move |result| Message::InstallationFinished(name.clone(), result),
    )
}

pub fn uninstall_hud(hud_info: &HudInfo, huds_directory: PathBuf) -> Command<Message> {
    if let Install::Installed { path, .. } = &hud_info.install {
        let hud_name = hud_info.name.clone();
        let hud_path = path.clone();

        Command::perform(
            async move { uninstall(&hud_path, huds_directory).await },
            move |result| match result {
                Ok(()) => Message::UninstallationFinished(hud_name),
                Err(error) => Message::error(format!("Failed to uninstall HUD '{0}'", hud_name), error),
            },
        )
    } else {
        Command::none()
    }
}
