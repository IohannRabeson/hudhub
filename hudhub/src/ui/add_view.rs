use crate::ui::DEFAULT_SPACING;
use crate::{AddContext, AddViewMessage, Message};
use hudhub_core::Source;
use iced::widget::{button, column, container, horizontal_space, row, text, text_input};
use iced::{Alignment, Element, Length};

pub fn add_view(context: &AddContext) -> Element<Message> {
    let mut add_button = button("Add HUB!");

    if context.is_form_valid {
        add_button = add_button.on_press(Message::ScanPackageToAdd(Source::DownloadUrl(context.download_url.clone())));
    }

    let input = row![
        horizontal_space(Length::Fill),
        text_input("Enter a download url", &context.download_url, |text| Message::AddView(
            AddViewMessage::DownloadUrlChanged(text)
        ))
        .id(context.download_url_text_input.clone())
        .width(Length::FillPortion(3)),
        horizontal_space(Length::Fill)
    ];

    let mut main_column = column![input]
        .align_items(Alignment::Center)
        .spacing(DEFAULT_SPACING);

    if let Some(error) = context.error.as_ref() {
        main_column = main_column.push(text(error))
    }

    main_column = main_column.push(add_button);

    container(main_column).height(Length::Fill).center_y().into()
}
