use crate::ff_helpers::PreviewVideo;
use crate::message::*;
use crate::views;

use iced::{Element, Task, Theme};

#[derive(Default)]
pub(crate) enum AppState {
    #[default]
    Initial,
    VideoOverview,
}

#[derive(Default)]
pub(crate) struct HLSenpai {
    pub video: Option<PreviewVideo>,
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
            AppState::VideoOverview => views::video_overview(self),
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }
}
