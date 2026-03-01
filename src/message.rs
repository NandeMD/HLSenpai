use crate::app::HLSenpai;
use crate::ff_helpers::validate_video_file;
use iced::Task;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectFilePressed,
}

pub(crate) fn handle_messages(app: &mut HLSenpai, message: Message) -> Task<Message> {
    match message {
        Message::SelectFilePressed => {
            let selection = rfd::FileDialog::new()
                .set_title("Select a video file")
                .add_filter(
                    "Video",
                    &["mp4", "mov", "mkv", "m4v", "avi", "webm", "ts", "m2ts"],
                )
                .pick_file();

            app.status = match selection {
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
