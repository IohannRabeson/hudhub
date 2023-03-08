use std::ffi::OsString;
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
use crate::paths::{DefaultPathsProvider, TestPathsProvider, PathsProvider};

mod commands;
mod state;
mod ui;
mod paths;

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
pub enum ListViewMessage {
    HudClicked(HudName),
    RemoveHud(HudName),
}

#[derive(Clone, Debug)]
pub enum Message {
    AddView(AddViewMessage),
    ListView(ListViewMessage),
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
    Back,
}

impl Message {
    pub fn error(title: impl ToString, message: impl ToString) -> Self {
        Self::Error(title.to_string(), message.to_string())
    }
}

struct Application {
    views: Views<View>,
    state: State,
    selected_hud: Option<HudName>,
    is_loading: bool,
    paths_provider: Box<dyn PathsProvider>,
}

impl Application {
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

    fn process_list_view_message(&mut self, message: ListViewMessage) -> Command<Message> {
        match message {
            ListViewMessage::HudClicked(hud_name) => {
                self.selected_hud = Some(hud_name);
            }
            ListViewMessage::RemoveHud(hud_name) => {
                if let Some(selected_hud) = self.selected_hud.as_ref() {
                    if selected_hud == &hud_name {
                        self.selected_hud = None;
                    }
                }

                if let Some(info) = self.state.registry.remove(&hud_name) {
                    if let Some(huds_directory) = self.paths_provider.get_huds_directory() {
                        return commands::uninstall_hud(&info, huds_directory.to_path_buf());
                    }
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
        let paths_provider: Box<dyn PathsProvider> = match std::env::args().find(|arg| arg == "--testing-mode") {
            None => Box::new(DefaultPathsProvider::new()),
            Some(_) => {
                println!("Testing mode enabled!");
                Box::new(TestPathsProvider::new())
            },
        };
        let application_state_file_path = paths_provider.get_application_state_file_path();
        let huds_directory_path = paths_provider.get_huds_directory();

        (
            Self {
                views: Views::new(View::List),
                state: State::default(),
                selected_hud: None,
                is_loading: false,
                paths_provider,
            },
            Command::batch([
                commands::load_state(application_state_file_path),
                commands::scan_huds_directory(huds_directory_path),
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
            Message::ListView(message) => {
                return self.process_list_view_message(message);
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
                    save_state(self.state.clone(), self.paths_provider.get_application_state_file_path()),
                    window::close(),
                ])
            }
            Message::Install(hud_name) => {
                if let Some(info) = self.state.registry.get(&hud_name) {
                    if let Some(huds_directory) = self.paths_provider.get_huds_directory() {
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
                    if let Some(huds_directory) = self.paths_provider.get_huds_directory() {
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
            Message::Back => {
                self.views.pop();
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message, Renderer<Self::Theme>> {
        match self.views.current().expect("current view") {
            View::List => ui::list_view::view(&self.state.registry, self.selected_hud.as_ref(), self.is_loading),
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
