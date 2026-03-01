use crate::app::HLSenpai;
use crate::message::Message;
use iced::widget::{button, column, container, markdown, row, scrollable, slider, text};
use iced::{Alignment, Element, Length};
use iced_video_player::VideoPlayer;

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

pub(crate) fn video_overview(app: &HLSenpai) -> El<'_> {
    let content: El<'_> = match app.video.as_ref() {
        Some(video) => {
            let duration_secs = video.video.duration().as_secs_f64();
            let slider_value = video.position.clamp(0.0, duration_secs);

            let left_panel = column![
                text(format!(
                    "Video Preview ({})",
                    video
                        .metadata
                        .video_codec
                        .as_deref()
                        .unwrap_or("Unknown codec")
                ))
                .size(24),
                container(
                    VideoPlayer::new(&video.video)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Contain)
                        .on_end_of_stream(Message::EndOfStream)
                        .on_new_frame(Message::NewFrame),
                )
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill),
                container(
                    slider(0.0..=duration_secs, slider_value, Message::Seek)
                        .step(0.1)
                        .on_release(Message::SeekRelease),
                )
                .padding(iced::Padding::new(5.0).left(10.0).right(10.0)),
                row![
                    button(if video.video.paused() {
                        "Play"
                    } else {
                        "Pause"
                    })
                    .width(80.0)
                    .on_press(Message::TogglePause),
                    button(if video.video.looping() {
                        "Disable Loop"
                    } else {
                        "Enable Loop"
                    })
                    .width(120.0)
                    .on_press(Message::ToggleLoop),
                    text(format!(
                        "{} / {}",
                        format_time(slider_value),
                        format_time(duration_secs)
                    ))
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                ]
                .spacing(5)
                .align_y(iced::alignment::Vertical::Center)
                .padding(iced::Padding::new(10.0).top(0.0)),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill);

            let right_panel = container(scrollable(
                markdown::view(
                    video.metadata_markdown.items(),
                    iced::Theme::TokyoNightStorm,
                )
                .map(Message::MarkdownLinkClicked),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(14.0));

            row![
                container(left_panel)
                    .width(Length::FillPortion(3))
                    .height(Length::Fill),
                container(right_panel)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill)
            ]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
        None => column![text("No video loaded. Select a file to continue.").size(24),]
            .spacing(16)
            .align_x(Alignment::Center)
            .into(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn format_time(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}s", total_seconds / 60, total_seconds % 60)
}
