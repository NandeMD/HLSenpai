use crate::app::{AppState, HLSenpai};
use crate::ff_helpers::{
    PreviewVideo, extract_video_metadata, validate_video_file, video_metadata_markdown,
};
use iced::Task;
use iced::widget::markdown;
use iced_video_player::Video;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectFilePressed,
    TogglePause,
    ToggleLoop,
    Seek(f64),
    SeekRelease,
    EndOfStream,
    NewFrame,
    MarkdownLinkClicked(markdown::Uri),
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
                        let metadata = match extract_video_metadata(&path) {
                            Ok(metadata) => metadata,
                            Err(reason) => {
                                let err_msg = format!(
                                    "Could not read video metadata:\n{}\nReason: {}",
                                    path.display(),
                                    reason
                                );
                                let _ = rfd::MessageDialog::new()
                                    .set_title("Metadata Read Failed")
                                    .set_description(&err_msg)
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .show();
                                eprintln!("{err_msg}");
                                app.state = AppState::Initial;
                                return Task::none();
                            }
                        };
                        let metadata_markdown_content =
                            markdown::Content::parse(&video_metadata_markdown(&metadata));

                        let video_url = match url::Url::from_file_path(&path) {
                            Ok(url) => url,
                            Err(()) => {
                                let err_msg = format!(
                                    "Could not convert file path to URL:\n{}",
                                    path.display()
                                );
                                let _ = rfd::MessageDialog::new()
                                    .set_title("File Path Error")
                                    .set_description(&err_msg)
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .show();
                                eprintln!("{err_msg}");
                                app.state = AppState::Initial;
                                return Task::none();
                            }
                        };

                        match Video::new(&video_url) {
                            Ok(video) => {
                                println!("Selected file: {}", path.display());
                                app.state = AppState::VideoOverview;
                                Some(PreviewVideo {
                                    video,
                                    _path: path,
                                    metadata,
                                    metadata_markdown: metadata_markdown_content,
                                    position: 0.0,
                                    dragging: false,
                                })
                            }
                            Err(err) => {
                                let err_msg = format!(
                                    "Could not load selected video:\n{}\nReason: {}",
                                    path.display(),
                                    err
                                );
                                let _ = rfd::MessageDialog::new()
                                    .set_title("Video Load Failed")
                                    .set_description(&err_msg)
                                    .set_buttons(rfd::MessageButtons::Ok)
                                    .show();
                                eprintln!("{err_msg}");
                                app.state = AppState::Initial;
                                None
                            }
                        }
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
                        app.state = AppState::Initial;
                        None
                    }
                },
                None => {
                    eprintln!("File selection cancelled");
                    app.state = AppState::Initial;
                    None
                }
            };
        }
        Message::TogglePause => {
            if let Some(video) = app.video.as_mut() {
                video.video.set_paused(!video.video.paused());
            }
        }
        Message::ToggleLoop => {
            if let Some(video) = app.video.as_mut() {
                video.video.set_looping(!video.video.looping());
            }
        }
        Message::Seek(seconds) => {
            if let Some(video) = app.video.as_mut() {
                video.dragging = true;
                video.position = seconds;
                video.video.set_paused(true);
            }
        }
        Message::SeekRelease => {
            if let Some(video) = app.video.as_mut() {
                video.dragging = false;
                if let Err(err) = video
                    .video
                    .seek(Duration::from_secs_f64(video.position), false)
                {
                    eprintln!("Could not seek video: {err}");
                }
                video.video.set_paused(false);
            }
        }
        Message::EndOfStream => {
            println!("End of stream reached");
        }
        Message::NewFrame => {
            if let Some(video) = app.video.as_mut()
                && !video.dragging
            {
                video.position = video.video.position().as_secs_f64();
            }
        }
        Message::MarkdownLinkClicked(uri) => {
            println!("Markdown link clicked: {uri}");
        }
    }

    Task::none()
}
