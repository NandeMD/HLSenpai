use ffmpeg_next as ffmpeg;
use iced::widget::{button, column, container, text};
use iced::{Alignment, Element, Length, Task};
use std::path::Path;

fn main() -> iced::Result {
    iced::application(HLSenpai::new, HLSenpai::update, HLSenpai::view).run()
}

#[derive(Default)]
struct HLSenpai {
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    SelectFilePressed,
}

impl HLSenpai {
    pub fn new() -> Self {
        Self::default()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFilePressed => {
                let selection = rfd::FileDialog::new()
                    .set_title("Select a video file")
                    .add_filter(
                        "Video",
                        &["mp4", "mov", "mkv", "m4v", "avi", "webm", "ts", "m2ts"],
                    )
                    .pick_file();

                self.status = match selection {
                    Some(path) => match validate_video_file(&path) {
                        Ok(()) => format!("Selected file is valid: {}", path.display()),
                        Err(reason) => format!(
                            "Selected file is not a valid supported video:\n{}\nReason: {}",
                            path.display(),
                            reason
                        ),
                    },
                    None => "File selection cancelled".to_string(),
                };
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
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

fn validate_video_file(path: &Path) -> Result<(), String> {
    ffmpeg::init().map_err(|err| format!("Failed to initialize ffmpeg: {err}"))?;

    let input = ffmpeg::format::input(path)
        .map_err(|err| format!("ffmpeg could not open the selected file: {err}"))?;

    let video_stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or_else(|| "No video stream was found in the selected file.".to_string())?;

    let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|err| format!("Could not read video stream parameters: {err}"))?;

    codec_context
        .decoder()
        .video()
        .map_err(|err| format!("No supported video decoder found for this stream: {err}"))?;

    Ok(())
}
