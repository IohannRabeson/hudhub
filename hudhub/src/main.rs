use iced::{Application as IcedApplication, Command, Element, Renderer, Settings, Theme};
use iced::keyboard::KeyCode::Comma;
use iced_views::Views;
use tempdir::TempDir;
use hudhub_core::{HudName, Registry, Source};

enum View {
    List,
    Add(AddContext),
}

#[derive(Default)]
pub struct AddContext {
    download_url: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    ShowAdd,
    DownloadUrlChanged(String),
    AddHud(Source, Vec<HudName>),
    ScanHudToAdd(Source),
    Error(String),

}

struct Application {
    views: Views<View>,
    registry: Registry,
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
                registry: Registry::new(),
            },
            Command::none(),
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
            Message::AddHud(source, hud_names) => {
                for hud_name in hud_names.into_iter() {
                    self.registry.add(hud_name, source.clone());
                }

                if let Some(View::Add(context)) = self.views.current() {
                    self.views.pop();
                }
            }
            Message::ScanHudToAdd(source) => {
                return commands::scan_package(source)
            }
            Message::Error(error) => {
                println!("Error: {}", error)
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message, Renderer<Self::Theme>> {
        match self.views.current().expect("current view") {
            View::List => ui::list_view(&self.registry),
            View::Add(context) => ui::add_view(&context),
        }
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}

mod commands {
    use std::path::Path;
    use iced::Command;
    use tempdir::TempDir;
    use hudhub_core::{fetch_package, HudName, Source};
    use crate::Message;

    pub fn scan_package(source: Source) -> Command<Message> {
        Command::perform(
            async {
                if let Ok(temp_dir) = TempDir::new("fetch_package_name") {
                    if let Ok(package) = fetch_package(source.clone(), temp_dir.path()).await {
                        let hud_names: Vec<HudName> = package.hud_names().cloned().collect();

                        return Some((source, hud_names))
                    }
                }

                None
            }, |result|{
                match result {
                    None => {
                        Message::Error("Invalid package".into())
                    }
                    Some((source, hud_names)) => {
                        Message::AddHud(source, hud_names)
                    }
                }
            }
        )
    }
}

mod ui {
    use crate::{AddContext, Message};
    use hudhub_core::{Registry, Source};
    use iced::widget::{text, column, row, scrollable, button, text_input};
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
        row![
            text_input("Enter download url", &context.download_url, Message::DownloadUrlChanged),
            button("Add").on_press(Message::ScanHudToAdd(Source::DownloadUrl(context.download_url.clone()))),
        ].into()
    }
}

fn main() -> iced::Result {
    Application::run(Settings::default())
}
