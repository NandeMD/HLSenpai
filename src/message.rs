use crate::app::{AppState, HLSenpai};
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

            app.video = match selection {
                Some(path) => match validate_video_file(&path) {
                    Ok(()) => {
                        println!("Selected file: {}", path.display());
                        app.state = AppState::VideoOverview;
                        Some(path)
                    }
                    Err(reason) => {
                        let err_msg = format!(
                            "Selected file is not a valid supported video:\n{}\nReason: {}",
                            path.display(),
                            reason
                        );
                        let _ = rfd::MessageDialog::new()
                            .set_title("Format Not Supported")
                            .set_description(&err_msg)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                        eprintln!("{err_msg}");
                        None
                    }
                },
                None => {
                    eprintln!("File selection cancelled");
                    None
                }
            };
        }
    }

    Task::none()
}
