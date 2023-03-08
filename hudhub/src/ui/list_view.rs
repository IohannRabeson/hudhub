use crate::ui::{color, DEFAULT_SPACING};
use crate::{AddViewMessage, ListViewMessage, Message};
use hudhub_core::{HudInfo, HudName, Install, Registry, Source};
use iced::widget::{button, column, container, row, scrollable, text, vertical_space, Container, Scrollable};
use iced::{theme, Alignment, Background, Color, Element, Length, Theme};
use iced_aw::Spinner;
use crate::ui::color::brighter_by;

pub fn view<'a>(registry: &'a Registry, selected_hud: Option<&'a HudName>, is_loading: bool) -> Element<'a, Message> {
    row![
        hud_list(registry, selected_hud).width(Length::FillPortion(4)).height(Length::Fill),
        action_list(registry, selected_hud, is_loading).width(Length::Fill).height(Length::Fill)
    ]
    .spacing(DEFAULT_SPACING)
    .padding(DEFAULT_SPACING)
    .into()
}

fn action_list<'a>(registry: &'a Registry, selected_hud: Option<&'a HudName>, is_loading: bool) -> Container<'a, Message> {
    if is_loading {
        return container(Spinner::new())
            .style(theme::Container::Custom(Box::new(BoxContainer{})))
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill);
    }

    let mut content = column![];

    if let Some(selected_hud) = selected_hud {
        match registry.get(selected_hud) {
            None => {}
            Some(info) => match info.install {
                Install::None => {
                    content = content.push(button("Install").on_press(Message::Install(info.name.clone())));
                }
                Install::Installed { .. } => {
                    content = content.push(button("Uninstall").on_press(Message::Uninstall(info.name.clone())));
                }
                Install::Failed { .. } => {
                    content = content.push(button("Install").on_press(Message::Install(info.name.clone())));
                }
            },
        }

        content =
            content.push(button("Remove").on_press(Message::ListView(ListViewMessage::RemoveHud(selected_hud.clone()))));
    }

    content = content.push(vertical_space(Length::Fill));
    content = content.push(
        button(text("Add HUD").size(36))
            .padding(16)
            .on_press(Message::AddView(AddViewMessage::Show))
            .style(theme::Button::Positive),
    );

    container(
        content
            .spacing(DEFAULT_SPACING)
            .align_items(Alignment::Center)
            .width(Length::Fill),
    )
    .style(theme::Container::Custom(Box::new(BoxContainer{})))
    .padding(DEFAULT_SPACING)
    .width(Length::Fill)
}

struct BoxContainer;

impl container::StyleSheet for BoxContainer {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(brighter_by(style.palette().background, 0.02))),
            ..Default::default()
        }
    }
}

fn hud_list<'a>(registry: &'a Registry, selected_hud: Option<&'a HudName>) -> Container<'a, Message> {
    container(scrollable(
        registry.iter().fold(column![].spacing(DEFAULT_SPACING), |c, info| {
            c.push(hud_info_view(info, selected_hud == Some(&info.name)))
        }),
    )).style(theme::Container::Custom(Box::new(BoxContainer{}))).padding(DEFAULT_SPACING)
}

struct UnselectedInfoView;
struct SelectedInfoView;

impl button::StyleSheet for SelectedInfoView {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Default::default(),
            background: Some(Background::Color(style.palette().primary)),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: style.palette().text,
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);

        appearance.background = Some(Background::Color(color::brighter(style.palette().primary)));

        appearance
    }
}

impl button::StyleSheet for UnselectedInfoView {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Default::default(),
            background: Some(Background::Color(color::brighter(style.palette().background))),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: style.palette().text,
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);

        appearance.background = Some(Background::Color(color::brighter_by(style.palette().background, 0.2)));

        appearance
    }
}

fn hud_info_view(info: &HudInfo, is_selected: bool) -> Element<Message> {
    let mut button = button(row![text(&info.name)])
        .on_press(Message::ListView(ListViewMessage::HudClicked(info.name.clone())))
        .width(Length::Fill)
        .style(theme::Button::Custom(match is_selected {
            true => Box::new(SelectedInfoView {}),
            false => Box::new(UnselectedInfoView {}),
        }));

    if !is_selected {
        button = button.style(theme::Button::Custom(Box::new(UnselectedInfoView {})));
    }

    button.into()
}
