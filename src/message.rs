use crate::app::{
    AppState, AudioCodec, EncodeOptionsForm, HLSenpai, HlsPlaylistType, VideoCodecLib,
    VideoProfile, X264Preset,
};
use crate::ff_helpers::{
    PreviewVideo, extract_video_metadata, validate_video_file, video_metadata_markdown,
};
use iced::Task;
use iced::widget::markdown;
use iced_video_player::Video;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    SelectFilePressed,
    OpenEncodeOptions,
    BackToVideoOverview,
    TogglePause,
    ToggleLoop,
    Seek(f64),
    SeekRelease,
    EndOfStream,
    NewFrame,
    MarkdownLinkClicked(markdown::Uri),
    EncodeScaleWidthChanged(String),
    EncodeScaleHeightChanged(String),
    EncodeScaleLockToggled(bool),
    EncodeGopChanged(String),
    EncodeVideoCodecLibSelected(VideoCodecLib),
    EncodeProfileSelected(VideoProfile),
    EncodePresetSelected(X264Preset),
    EncodeScThresholdChanged(String),
    EncodeAudioCodecSelected(AudioCodec),
    EncodeAudioChannelsChanged(String),
    EncodeHlsTimeChanged(String),
    EncodePlaylistTypeSelected(HlsPlaylistType),
    EncodeIndependentSegmentsToggled(bool),
    EncodeMasterPlaylistNameChanged(String),
    EncodeSegmentPatternChanged(String),
    EncodeOutputPlaylistPatternChanged(String),
    EncodeOutputBaseFolderChanged(String),
    EncodeOutputSubfolderChanged(String),
    EncodeOutputMasterPlaylistFileChanged(String),
    EncodePickOutputBaseFolderPressed,
    EncodePickOutputMasterPlaylistFilePressed,
    EncodeVariantNameChanged(usize, String),
    EncodeVariantVideoBitrateChanged(usize, String),
    EncodeVariantMaxrateChanged(usize, String),
    EncodeVariantBufsizeChanged(usize, String),
    EncodeVariantAudioBitrateChanged(usize, String),
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
                                app.encode_options = None;
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
                                app.encode_options = None;
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
                        app.encode_options = None;
                        None
                    }
                },
                None => {
                    eprintln!("File selection cancelled");
                    app.state = AppState::Initial;
                    app.encode_options = None;
                    None
                }
            };
        }
        Message::OpenEncodeOptions => {
            if let Some(video) = app.video.as_ref() {
                if app.encode_options.is_none() {
                    let mut form = EncodeOptionsForm::from_metadata(&video.metadata);
                    if let Some(stem) = video_stem(&video._path) {
                        form.output_subfolder_name = stem;
                    }
                    app.encode_options = Some(form);
                }
                app.state = AppState::EncodeOptions;
            }
        }
        Message::BackToVideoOverview => {
            if app.video.is_some() {
                app.state = AppState::VideoOverview;
            } else {
                app.state = AppState::Initial;
            }
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
        Message::EncodeScaleWidthChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_u32(&value)
            {
                form.set_scale_width(parsed.max(1));
            }
        }
        Message::EncodeScaleHeightChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_u32(&value)
            {
                form.set_scale_height(parsed.max(1));
            }
        }
        Message::EncodeScaleLockToggled(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.set_scale_lock_aspect(value);
            }
        }
        Message::EncodeGopChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_u32(&value)
            {
                form.gop = parsed.max(1);
            }
        }
        Message::EncodeVideoCodecLibSelected(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.apply_codec_defaults(value);
            }
        }
        Message::EncodeProfileSelected(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.profile = value;
            }
        }
        Message::EncodePresetSelected(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.preset = value;
            }
        }
        Message::EncodeScThresholdChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_i32(&value)
            {
                form.sc_threshold = parsed.max(0);
            }
        }
        Message::EncodeAudioCodecSelected(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.audio_codec = value;
            }
        }
        Message::EncodeAudioChannelsChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_u8(&value)
            {
                form.audio_channels = parsed.max(1);
            }
        }
        Message::EncodeHlsTimeChanged(value) => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(parsed) = parse_u32(&value)
            {
                form.hls_time_seconds = parsed.max(1);
            }
        }
        Message::EncodePlaylistTypeSelected(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.hls_playlist_type = value;
            }
        }
        Message::EncodeIndependentSegmentsToggled(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.hls_flags_independent_segments = value;
            }
        }
        Message::EncodeMasterPlaylistNameChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.master_playlist_name = value;
            }
        }
        Message::EncodeSegmentPatternChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.segment_filename_pattern = value;
            }
        }
        Message::EncodeOutputPlaylistPatternChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.output_variant_playlist_pattern = value;
            }
        }
        Message::EncodeOutputBaseFolderChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.output_base_folder = value;
            }
        }
        Message::EncodeOutputSubfolderChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.output_subfolder_name = value;
            }
        }
        Message::EncodeOutputMasterPlaylistFileChanged(value) => {
            if let Some(form) = app.encode_options.as_mut() {
                form.output_master_playlist_file = value;
            }
        }
        Message::EncodePickOutputBaseFolderPressed => {
            if let Some(form) = app.encode_options.as_mut()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Select output base folder")
                    .pick_folder()
            {
                form.output_base_folder = path.display().to_string();
            }
        }
        Message::EncodePickOutputMasterPlaylistFilePressed => {
            if let Some(form) = app.encode_options.as_mut() {
                let current_name = form.output_master_playlist_file.clone();

                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select master playlist file")
                    .set_file_name(&current_name)
                    .add_filter("HLS playlists", &["m3u8"])
                    .save_file()
                {
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        form.output_master_playlist_file = file_name.to_string();
                    }

                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        form.output_base_folder = parent.display().to_string();
                    }
                }
            }
        }
        Message::EncodeVariantNameChanged(index, value) => {
            if let Some(variant) = app
                .encode_options
                .as_mut()
                .and_then(|form| form.variants.get_mut(index))
            {
                variant.name = value;
            }
        }
        Message::EncodeVariantVideoBitrateChanged(index, value) => {
            if let Some(parsed) = parse_u32(&value)
                && let Some(variant) = app
                    .encode_options
                    .as_mut()
                    .and_then(|form| form.variants.get_mut(index))
            {
                variant.video_bitrate_k = parsed.max(1);
            }
        }
        Message::EncodeVariantMaxrateChanged(index, value) => {
            if let Some(parsed) = parse_u32(&value)
                && let Some(variant) = app
                    .encode_options
                    .as_mut()
                    .and_then(|form| form.variants.get_mut(index))
            {
                variant.maxrate_k = parsed.max(1);
            }
        }
        Message::EncodeVariantBufsizeChanged(index, value) => {
            if let Some(parsed) = parse_u32(&value)
                && let Some(variant) = app
                    .encode_options
                    .as_mut()
                    .and_then(|form| form.variants.get_mut(index))
            {
                variant.bufsize_k = parsed.max(1);
            }
        }
        Message::EncodeVariantAudioBitrateChanged(index, value) => {
            if let Some(parsed) = parse_u32(&value)
                && let Some(variant) = app
                    .encode_options
                    .as_mut()
                    .and_then(|form| form.variants.get_mut(index))
            {
                variant.audio_bitrate_k = parsed.max(1);
            }
        }
    }

    Task::none()
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn parse_u8(value: &str) -> Option<u8> {
    value.trim().parse::<u8>().ok()
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

fn video_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
