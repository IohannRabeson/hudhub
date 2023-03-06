use std::path::PathBuf;
use iced::{Application as IcedApplication, Command, Element, event, Renderer, Settings, subscription, Subscription, Theme, window};
use iced::keyboard::KeyCode::Comma;
use iced_views::Views;
use tempdir::TempDir;
use hudhub_core::{HudName, Registry, Source};
use state::State;
use crate::commands::save_state;
use platform_dirs::AppDirs;

mod state;

enum View {
    List,
    Add(AddContext),
}

#[derive(Default)]
pub struct AddContext {
    download_url: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Message {
    ShowAdd,
    DownloadUrlChanged(String),
    AddHuds(Source, Vec<HudName>),
    ScanPackageToAdd(Source),
    Error(String, String),
    StateSaved,
    StateLoaded(State),
    Quit,
}

impl Message {
    pub fn error(title: impl ToString, message: impl ToString) -> Self {
        Self::Error(
            title.to_string(),
            message.to_string(),
        )
    }
}

struct Application {
    views: Views<View>,
    state: State,
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

        steamdir.app(&TEAMFORTRESS2_STEAMAPPID).map(|dir|dir.path.clone())
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
            },
            commands::load_state(Self::get_application_state_file_path()),
        )
    }

    fn title(&self) -> String {
        "HudHub".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ShowAdd => {
                self.views.push(View::Add(AddContext::default()))
            },
            Message::DownloadUrlChanged(url) => {
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.download_url = url;
                }
            }
            Message::AddHuds(source, hud_names) => {
                for hud_name in hud_names.into_iter() {
                    self.state.registry.add(hud_name, source.clone());
                }

                if let Some(View::Add(_context)) = self.views.current() {
                    self.views.pop();
                }
            }
            Message::ScanPackageToAdd(source) => {
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.error = None;
                }
                return commands::scan_package(source)
            }
            Message::Error(title, error) => {
                println!("{}: {}", title, error);
                if let Some(View::Add(context)) = self.views.current_mut() {
                    context.error = Some(error);
                }
            }
            Message::StateSaved => {}
            Message::StateLoaded(state) => { self.state = state; }
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
            View::List => ui::list_view(&self.state.registry),
            View::Add(context) => ui::add_view(&context),
        }
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
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

mod commands {
    use std::path::{Path, PathBuf};
    use iced::Command;
    use tempdir::TempDir;
    use hudhub_core::{fetch_package, FetchError, HudName, Source};
    use crate::Message;
    use crate::state::State;

    #[derive(thiserror::Error, Debug)]
    enum ScanPackageError {
        #[error(transparent)]
        FetchPackageFailed(#[from] FetchError),

        #[error("Failed to create a temporary directory: {0}")]
        FailedToCreateTempDirectory(std::io::Error),
    }

    async fn get_hud_names(source: Source) -> Result<Vec<HudName>, ScanPackageError> {
        let temp_directory = TempDir::new("fetch_package_name").map_err(ScanPackageError::FailedToCreateTempDirectory)?;
        let package = fetch_package(source.clone(), temp_directory.path()).await?;

        Ok(package.hud_names().cloned().collect())
    }

    pub fn scan_package(source: Source) -> Command<Message> {
        let source_for_future = source.clone();

        Command::perform(
            async move {
                get_hud_names(source_for_future).await
            }, move |result|{
                match result {
                    Err(error) => {
                        Message::error("Failed to scan package", error)
                    }
                    Ok(hud_names) => {
                        Message::AddHuds(source, hud_names)
                    }
                }
            }
        )
    }

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
}

mod ui {
    use crate::{AddContext, Message};
    use hudhub_core::{Registry, Source};
    use iced::widget::{button, column, row, scrollable, text, text_input};
    use iced::Element;

    pub fn list_view(registry: &Registry) -> Element<Message> {
        column![
            button("Add").on_press(Message::ShowAdd),
            scrollable(registry
            .iter()
            .fold(column![], |c, info| c.push(row![text(&info.name)])))
        ]
        .into()
    }

    pub fn add_view(context: &AddContext) -> Element<Message> {
        let mut main_column = column![
            row![
                text_input("Enter download url", &context.download_url, Message::DownloadUrlChanged),
                button("Add").on_press(Message::ScanPackageToAdd(Source::DownloadUrl(context.download_url.clone()))),
            ]
        ];

        if let Some(error) = context.error.as_ref() {
            main_column = main_column.push(text(error))
        }

        main_column.into()
    }
}

fn main() -> iced::Result {
    let mut settings = Settings::default();

    settings.exit_on_close_request = false;

    Application::run(settings)
}
