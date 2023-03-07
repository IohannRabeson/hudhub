use crate::{AddViewMessage, Message};
use hudhub_core::{HudInfo, Install, Registry, Source};
use iced::widget::{button, column, row, scrollable, text};
use iced::Element;

pub mod add_view;

const DEFAULT_SPACING: u16 = 8;

pub fn list_view(registry: &Registry, is_loading: bool) -> Element<Message> {
    column![
        button("Add").on_press(Message::AddView(AddViewMessage::Show)),
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
