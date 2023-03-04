use crate::huds::{ArchiveLocation, Install};
use iced::{Application as IcedApplication, Command, Element, event, Renderer, Subscription, subscription, window};
use std::path::{Path, PathBuf};
use std::time::Instant;
use chrono::Utc;
use iced::event::Status;
use iced_aw::Modal;
use platform_dirs::AppDirs;
use crate::huds::hud::Hud;
use serde::{Deserialize, Serialize};
use state::State;
use crate::commands::save_state;

mod huds;
mod views;
mod state;
mod ui;
mod archive;

#[derive(Debug, Clone)]
pub enum ViewMessage {
    ShowAddView,
    CloseCurrentView,
}

#[derive(Debug, Clone)]
pub enum AddMessage {
    DownloadUrlChanged(String),
    NameChanged(String),
    Add(String, String)
}

#[derive(Debug, Clone)]
pub enum HudMessage {
    Install(ArchiveLocation),
    Uninstall(ArchiveLocation),
    Installed(ArchiveLocation, PathBuf),
    Uninstalled(ArchiveLocation),
    InstallationFailed(ArchiveLocation, String),
    UninstallationFailed(ArchiveLocation, String),
}

#[derive(Debug, Clone)]
pub enum Message {
    View(ViewMessage),
    Add(AddMessage),
    Hud(HudMessage),
    StateLoaded(State),
    StateSaved,
    Error(String, String),
    Quit,
}

impl Message {
    pub fn error(context: impl ToString, message: impl ToString) -> Message {
        Message::Error(context.to_string(), message.to_string())
    }
}

struct Application {
    views: views::Views<Screens>,
    state: State,
    /// The Team Fortress 2 directory.
    /// E.g: `..\Steam\steamapps\common\Team Fortress 2`
    team_fortress_directory: Option<PathBuf>,
}

mod commands {
    use std::io::Error;
    use std::path::{Path, PathBuf};
    use iced::Command;
    use crate::huds::{archive_location, ArchiveLocation, InstallError, UninstallError};
    use crate::{archive, HudMessage, Message};
    use crate::state::State;

    pub fn save_state(state: State, path: impl Into<PathBuf>) -> Command<Message> {
        let path = path.into();

        println!("Save state: {}", path.display());

        Command::perform(async move {
            State::save(&state, &path).await
        }, |result| {
            match result {
                Ok(()) => { Message::StateSaved }
                Err(error) => { Message::error("Failed to save application state", error) }
            }
        })
    }

    pub fn load_state(path: impl Into<PathBuf>) -> Command<Message> {
        let path = path.into();

        println!("Load state: {}", path.display());

        Command::perform(async move {
            State::load(&path).await
        }, |result| {
            match result {
                Ok(state) => { Message::StateLoaded(state) }
                Err(error) => { Message::error("Failed to load application state", error) }
            }
        })
    }

    pub fn install(source: ArchiveLocation, huds_directory: PathBuf) -> Command<Message> {
        Command::perform(install_hud(source.clone(), huds_directory),move |result|{
            match result {
                Ok(installation_path) => Message::Hud(HudMessage::Installed(source.clone(), installation_path)),
                Err(error) => Message::Hud(HudMessage::InstallationFailed(source.clone(), error.to_string())),
            }
        })
    }

    pub fn uninstall(source: ArchiveLocation, install_path: PathBuf, huds_directory: PathBuf) -> Command<Message> {
        Command::perform(uninstall_hud(install_path.clone(), huds_directory),move |result|{
            match result {
                Ok(()) => Message::Hud(HudMessage::Uninstalled(source.clone())),
                Err(error) => Message::Hud(HudMessage::UninstallationFailed(source.clone(), error.to_string())),
            }
        })
    }

    async fn install_hud(source: ArchiveLocation, huds_directory: PathBuf) -> Result<PathBuf, InstallError> {
        tokio::fs::create_dir_all(&huds_directory)
            .await
            .map_err(|e| InstallError::FailedToCreateDirectory(e, huds_directory.to_path_buf()))?;

        let temp_directory = tempdir::TempDir::new("tf2-huds-manager-temporary")
            .map_err(|e| InstallError::FailedToCreateDirectory(e, huds_directory.to_path_buf()))?;
        let archive_path = archive_location::fetch_archive(&source, temp_directory.path()).await?;

        archive::extract(&archive_path, &huds_directory).map_err(InstallError::FailedToExtractHud)
    }

    async fn uninstall_hud(install_path: PathBuf, huds_directory: PathBuf) -> Result<(), UninstallError> {
        assert!(install_path.starts_with(huds_directory));

        tokio::fs::remove_dir_all(&install_path)
            .await
            .map_err(|e| UninstallError::FailedToRemoveDirectory(e, install_path))?;

        Ok(())
    }
}

impl Application {
    fn get_application_directory() -> PathBuf {
        let app_dirs = AppDirs::new(Some("tf2-hubs-manager"), false).unwrap();

        app_dirs.config_dir
    }

    fn get_application_state_file_path() -> PathBuf {
        Self::get_application_directory().join("application.state")
    }

    fn get_team_fortress_directory() -> Option<PathBuf> {
        let mut steamdir = steamlocate::SteamDir::locate().unwrap();
        const TEAMFORTRESS2_STEAMAPPID: u32 = 440;

        steamdir.app(&TEAMFORTRESS2_STEAMAPPID).map(|dir|dir.path.clone())
    }
}

impl IcedApplication for Application {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                views: views::Views::new(Screens::List),
                state: State::new(),
                team_fortress_directory: Self::get_team_fortress_directory(),
            },
            commands::load_state(Self::get_application_state_file_path()),
        )
    }

    fn title(&self) -> String {
        "TF2 HUDs Manager".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::View(message) => {
                match message {
                    ViewMessage::ShowAddView => {
                        self.views.push(Screens::Add(AddContext::default()));
                    }
                    ViewMessage::CloseCurrentView => {
                        self.views.pop();
                    }
                }
            },
            Message::Add(message) => {
                if let Some(Screens::Add(context)) = self.views.current_mut() {
                    match message {
                        AddMessage::DownloadUrlChanged(url) => {
                            context.download_url = url;
                        }
                        AddMessage::NameChanged(name) => {
                            context.name = name;
                        }
                        AddMessage::Add(name, url) => {
                            self.state.huds.add(Hud{
                                archive_location: ArchiveLocation::DownloadUrl(url),
                                display_name: name,
                            });
                            self.views.pop();
                        }
                    }
                }
            }
            Message::Hud(message) => {
                match message {
                    HudMessage::Install(key) => {
                        if let Some(team_fortress_directory) = self.team_fortress_directory.as_ref() {
                            let huds_directory = team_fortress_directory.join("tf").join("custom");
                            if let Some(info) = self.state.huds.get(&key) {
                                if !info.install.is_installed() {
                                    return commands::install(key, huds_directory)
                                }
                            }
                        }
                    }
                    HudMessage::Uninstall(key) => {
                        if let Some(team_fortress_directory) = self.team_fortress_directory.as_ref() {
                            if let Some(info) = self.state.huds.get(&key) {
                                if let Install::Installed { path, .. } = &info.install {
                                    let huds_directory = team_fortress_directory.join("tf").join("custom");

                                    return commands::uninstall(key, path.clone(), huds_directory)
                                }
                            }
                        }
                    }
                    HudMessage::Installed(key, installation_path) => {
                        self.state.huds.set_install(&key, Install::Installed {
                            path: installation_path,
                            timestamp: Utc::now(),
                        });
                    }
                    HudMessage::InstallationFailed(key, error) => {
                        self.state.huds.set_install(&key, Install::Failed {
                            error,
                        });
                    }
                    HudMessage::Uninstalled(key) => {
                        self.state.huds.set_install(&key, Install::None);
                    }
                    HudMessage::UninstallationFailed(key, error) => {

                    }
                }
            }
            Message::StateLoaded(state) => {
                self.state = state;
            }
            Message::StateSaved => {}
            Message::Error(context, message) => {
                eprintln!("{}: {}", context, message);
            }
            Message::Quit => {
                return Command::batch([
                    save_state(self.state.clone(), Self::get_application_state_file_path()),
                    window::close(),
                ])
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message, Renderer<Self::Theme>> {
        match self.views.current().expect("current view") {
            Screens::List => ui::list_view(&self.state.huds),
            Screens::Add(context) => {
                let background = ui::list_view(&self.state.huds);

                Modal::new(true, background, || ui::add_view(context)).into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscription::events_with(|event, _status | {
            if let event::Event::Window(window::Event::CloseRequested) = event {
                return Some(Message::Quit)
            }

            None
        })
    }
}

#[derive(Default)]
pub struct AddContext {
    pub download_url: String,
    pub name: String,
}

enum Screens {
    /// The list of user's HUBs
    List,
    /// The view to add a new HUB to the user list.
    Add(AddContext),
}

fn main() -> iced::Result {
    let mut settings = iced::Settings::default();

    settings.exit_on_close_request = false;

    Application::run(settings)
}
