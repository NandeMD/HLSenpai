use crate::app::HLSenpai;
use crate::message::Message;
use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length};

type El<'a> = Element<'a, Message>;

pub(crate) fn select_file(_app: &HLSenpai) -> El<'_> {
    let content = column![button("Select File").on_press(Message::SelectFilePressed),]
        .spacing(16)
        .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub(crate) fn video_overview(_app: &HLSenpai) -> El<'_> {
    let content = column![text("Video Overview - Coming Soon!").size(24),]
        .spacing(16)
        .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
