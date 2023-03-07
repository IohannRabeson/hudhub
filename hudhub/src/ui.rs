use crate::{AddContext, Message};
use hudhub_core::{HudInfo, Install, Registry, Source};
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::Element;

pub fn list_view(registry: &Registry, is_loading: bool) -> Element<Message> {
    column![
        button("Add").on_press(Message::ShowAdd),
        scrollable(
            registry
                .iter()
                .fold(column![], |c, info| c.push(hud_info_view(info, is_loading)))
        )
    ]
    .into()
}

fn hud_info_view(info: &HudInfo, is_loading: bool) -> Element<Message> {
    let mut install_button = button("Install");
    let mut uninstall_button = button("Uninstall");

    if !is_loading {
        if info.source != Source::None {
            install_button = install_button.on_press(Message::Install(info.name.clone()));
        }
        uninstall_button = uninstall_button.on_press(Message::Uninstall(info.name.clone()));
    }

    match info.install {
        Install::None => {
            row![text(&info.name), install_button]
        }
        Install::Installed { .. } => {
            row![text(&info.name), uninstall_button]
        }
        Install::Failed { .. } => {
            row![text(&info.name)]
        }
    }
    .into()
}

pub fn add_view(context: &AddContext) -> Element<Message> {
    let mut main_column = column![row![
        text_input("Enter download url", &context.download_url, Message::DownloadUrlChanged),
        button("Add").on_press(Message::ScanPackageToAdd(Source::DownloadUrl(context.download_url.clone()))),
    ]];

    if let Some(error) = context.error.as_ref() {
        main_column = main_column.push(text(error))
    }

    main_column.into()
}
