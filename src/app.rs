use crate::message::*;
use crate::views;

use iced::{Element, Task, Theme};

#[derive(Default)]
pub(crate) enum AppState {
    #[default]
    Initial,
}

#[derive(Default)]
pub(crate) struct HLSenpai {
    pub status: String,
    pub state: AppState,
}

impl HLSenpai {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        handle_messages(self, message)
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        match self.state {
            AppState::Initial => views::select_file(self),
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }
}
