use crate::commands::save_state;
use hudhub_core::{HudDirectory, HudName, Install, Source, Url};
use iced::widget::text_input;
use iced::{
    event, subscription, window, Application as IcedApplication, Command, Element, Renderer, Settings, Subscription, Theme,
};
use iced_views::Views;
use platform_dirs::AppDirs;
use state::State;
use std::path::PathBuf;
use ui::add_view;

mod commands;
mod state;
mod ui;

enum View {
    List,
    Add(AddContext),
}

pub struct AddContext {
    download_url: String,
    is_form_valid: bool,
    error: Option<String>,
    download_url_text_input: text_input::Id,
    scanning: bool,
}

impl Default for AddContext {
    fn default() -> Self {
        Self {
            download_url: String::new(),
            is_form_valid: false,
            error: None,
            download_url_text_input: text_input::Id::unique(),
            scanning: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AddViewMessage {
    Show,
    DownloadUrlChanged(String),
    ScanPackageToAdd(Source),
}

#[derive(Clone, Debug)]
pub enum Message {
    AddView(AddViewMessage),
    AddHuds(Source, Vec<HudName>),
    Install(HudName),
    Uninstall(HudName),
    Error(String, String),
    StateSaved,
    StateLoaded(State),
    InstallationFinished(HudName, Install),
    UninstallationFinished(HudName),
    FoundInstalledHuds(Vec<HudDirectory>),
    Quit,
}

impl Message {
    pub fn error(title: impl ToString, message: impl ToString) -> Self {
        Self::Error(title.to_string(), message.to_string())
    }
}

struct Application {
    views: Views<View>,
    state: State,
    is_loading: bool,
}

impl Application {
    fn get_application_directory() -> PathBuf {
        let app_dirs = AppDirs::new(Some("hudhub"), false).unwrap();

        app_dirs.config_dir
    }

    fn get_application_state_file_path() -> PathBuf {
        Self::get_application_directory().join("application.state")
    }

    fn get_team_fortress_directory() -> Option<PathBuf> {
        let mut steamdir = steamlocate::SteamDir::locate().unwrap();
        const TEAMFORTRESS2_STEAMAPPID: u32 = 440;

        steamdir.app(&TEAMFORTRESS2_STEAMAPPID).map(|dir| dir.path.clone())
    }

    fn get_team_fortress_huds_directory() -> Option<PathBuf> {
        Self::get_team_fortress_directory().map(|directory| directory.join("tf").join("custom"))
    }

    fn process_add_view_message(&mut self, message: AddViewMessage) -> Command<Message> {
        match message {
            AddViewMessage::Show => {
                let context = AddContext::default();
                let focus_command = text_input::focus(context.download_url_text_input.clone());

                self.views.push(View::Add(context));

                return focus_command;
            }
            AddViewMessage::DownloadUrlChanged(url) => {
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.download_url = url.clone();
                    context.is_form_valid = if !url.is_empty() {
                        match Url::parse(&url) {
                            Ok(_) => true,
                            Err(error) => {
                                context.error = Some(format!("Invalid URL: {}", error));
                                false
                            }
                        }
                    } else {
                        context.error = None;
                        false
                    };
                }
            }
            AddViewMessage::ScanPackageToAdd(source) => {
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.error = None;
                    context.scanning = true;
                    return commands::scan_package(source);
                }
            }
        }

        Command::none()
    }
}

impl IcedApplication for Application {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                views: Views::new(View::List),
                state: State::default(),
                is_loading: false,
            },
            Command::batch([
                commands::load_state(Self::get_application_state_file_path()),
                commands::scan_huds_directory(Self::get_team_fortress_huds_directory()),
            ]),
        )
    }

    fn title(&self) -> String {
        "HudHub".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::AddView(message) => {
                return self.process_add_view_message(message);
            }
            Message::AddHuds(source, hud_names) => {
                for hud_name in hud_names.into_iter() {
                    self.state.registry.add(hud_name, source.clone());
                }

                if let Some(View::Add(_context)) = self.views.current() {
                    self.views.pop();
                }
            }
            Message::Error(title, error) => {
                println!("{}: {}", title, error);
                self.is_loading = false;
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.error = Some(error);
                    context.scanning = false;
                }
            }
            Message::StateSaved => {}
            Message::StateLoaded(state) => {
                self.state = state;
            }
            Message::Quit => {
                return Command::batch([
                    save_state(self.state.clone(), Self::get_application_state_file_path()),
                    window::close(),
                ])
            }
            Message::Install(hud_name) => {
                if let Some(info) = self.state.registry.get(&hud_name) {
                    if let Some(huds_directory) = Self::get_team_fortress_huds_directory() {
                        assert!(!matches!(info.install, Install::Installed { .. }));

                        let mut commands = Vec::new();

                        if let Some(installed_info) = self.state.registry.get_installed() {
                            commands.push(commands::uninstall_hud(installed_info, huds_directory.clone()));
                        }

                        commands.push(commands::install_hud(info.source.clone(), hud_name, huds_directory));

                        self.is_loading = true;

                        return Command::batch(commands.into_iter());
                    }
                }
            }
            Message::Uninstall(hud_name) => {
                if let Some(info) = self.state.registry.get(&hud_name) {
                    if let Some(huds_directory) = Self::get_team_fortress_huds_directory() {
                        assert!(matches!(info.install, Install::Installed { .. }));
                        self.is_loading = true;
                        return commands::uninstall_hud(&info, huds_directory);
                    }
                }
            }
            Message::InstallationFinished(hud_name, install) => {
                self.state.registry.set_install(&hud_name, install);
                self.is_loading = false;
            }
            Message::UninstallationFinished(hud_name) => {
                self.state.registry.set_install(&hud_name, Install::None);
                self.is_loading = false;
            }
            Message::FoundInstalledHuds(hud_directories) => {
                for hud_directory in hud_directories {
                    if let Some(info) = self.state.registry.get(&hud_directory.name) {
                        if let Install::Installed { path, .. } = &info.install {
                            if path != &hud_directory.path {
                                self.state
                                    .registry
                                    .set_install(&hud_directory.name, Install::installed_now(path));
                            }
                        }
                    } else {
                        self.state.registry.add(hud_directory.name.clone(), Source::None);
                        self.state
                            .registry
                            .set_install(&hud_directory.name, Install::installed_now(&hud_directory.path));
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message, Renderer<Self::Theme>> {
        match self.views.current().expect("current view") {
            View::List => ui::list_view(&self.state.registry, self.is_loading),
            View::Add(context) => add_view::add_view(&context),
        }
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscription::events_with(|event, _status| {
            if let event::Event::Window(window::Event::CloseRequested) = event {
                return Some(Message::Quit);
            }

            None
        })
    }
}

fn main() -> iced::Result {
    let mut settings = Settings::default();

    settings.exit_on_close_request = false;

    Application::run(settings)
}
