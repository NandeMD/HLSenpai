use crate::app::HLSenpai;
use crate::message::Message;
use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length};

type El<'a> = Element<'a, Message>;

pub(crate) fn select_file(app: &HLSenpai) -> El<'_> {
    let content = column![
        button("Select File").on_press(Message::SelectFilePressed),
        text(if app.status.is_empty() {
            "Pick a file to validate with ffmpeg"
        } else {
            &app.status
        })
    ]
    .spacing(16)
    .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
