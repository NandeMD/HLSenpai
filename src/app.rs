use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length, Task};

use crate::message::*;

#[derive(Default)]
pub(crate) struct HLSenpai {
    pub status: String,
}

impl HLSenpai {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        handle_messages(self, message)
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let content = column![
            button("Select File").on_press(Message::SelectFilePressed),
            text(if self.status.is_empty() {
                "Pick a file to validate with ffmpeg"
            } else {
                &self.status
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
}
