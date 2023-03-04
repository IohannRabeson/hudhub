use crate::huds::{HudInfo, Huds, Install};
use crate::{AddContext, AddMessage, HudMessage, Message, ViewMessage};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::Element;
use crate::huds::hud::Hud;

fn hud_view(info: &HudInfo) -> Element<Message> {
    row![
        text(&info.hud.display_name),
        button("Install").on_press(Message::Hud(HudMessage::Install(info.hud.archive_location.clone()))),
        button("Uninstall").on_press(Message::Hud(HudMessage::Uninstall(info.hud.archive_location.clone()))),
    ].into()
}

pub fn list_view(huds: &Huds) -> Element<Message> {
    let list = scrollable(huds.iter()
        .fold(column![], |column, info|{
            column.push(hud_view(info))
        })
    );

    column![
        button("Add").on_press(Message::View(ViewMessage::ShowAddView)),
        list
    ].into()
}

pub fn add_view(context: &AddContext) -> Element<Message> {
    container(column![
        text_input("Name", &context.name, |name|Message::Add(AddMessage::NameChanged(name))),
        text_input("Download URL", &context.download_url, |url|Message::Add(AddMessage::DownloadUrlChanged(url))),
        button("Add").on_press(Message::Add(AddMessage::Add(context.name.clone(), context.download_url.clone())))
    ]).into()
}
