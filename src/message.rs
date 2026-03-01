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
use std::path::{Path, PathBuf};
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
    PrintFfmpegScript,
    CloseFfmpegScriptPopup,
}

pub(crate) fn handle_messages(app: &mut HLSenpai, message: Message) -> Task<Message> {
    match message {
        Message::SelectFilePressed => {
            app.ffmpeg_script_popup = None;

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
            app.ffmpeg_script_popup = None;

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
        Message::PrintFfmpegScript => {
            if let (Some(video), Some(form)) = (app.video.as_ref(), app.encode_options.as_ref()) {
                let script = build_ffmpeg_script(&video._path, form);
                let markdown_script = ffmpeg_script_popup_markdown(&script);
                app.ffmpeg_script_popup = Some(markdown::Content::parse(&markdown_script));
            } else {
                eprintln!("Cannot build ffmpeg script: no video or encode options are available.");
            }
        }
        Message::CloseFfmpegScriptPopup => {
            app.ffmpeg_script_popup = None;
        }
    }

    Task::none()
}

fn build_ffmpeg_script(input_path: &Path, form: &EncodeOptionsForm) -> String {
    let output_root = output_root_folder(form);
    let output_root_string = output_root.to_string_lossy().to_string();
    let segment_pattern =
        non_empty(&form.segment_filename_pattern).unwrap_or_else(|| "v%v/seg_%06d.ts".to_string());
    let variant_playlist_pattern = non_empty(&form.output_variant_playlist_pattern)
        .unwrap_or_else(|| "v%v/prog.m3u8".to_string());
    let master_playlist_name = non_empty(&form.output_master_playlist_file)
        .or_else(|| non_empty(&form.master_playlist_name))
        .unwrap_or_else(|| "master.m3u8".to_string());

    let segment_filename = output_root.join(segment_pattern);
    let variant_playlist = output_root.join(variant_playlist_pattern);

    let video_encoder = video_encoder_name(form.video_codec_lib);
    let audio_encoder = audio_encoder_name(form.audio_codec);
    let profile = ffmpeg_profile_value(form.video_codec_lib, form.profile);
    let preset_argument = codec_preset_argument(form.video_codec_lib, form.preset);

    let split_outputs = (0..form.variants.len())
        .map(|index| format!("[v{index}]"))
        .collect::<Vec<_>>()
        .join("");

    let scale_chains = (0..form.variants.len())
        .map(|index| {
            format!(
                "[v{index}]scale={}:{}:flags=lanczos[v{index}out]",
                form.scale_width, form.scale_height
            )
        })
        .collect::<Vec<_>>()
        .join(";");

    let filter_complex = format!(
        "[0:v]split={count}{split_outputs};{scale_chains}",
        count = form.variants.len()
    );

    let var_stream_map = form
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            format!(
                "v:{index},a:{index},name:{}",
                sanitize_variant_name(&variant.name, index)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut lines = vec![
        "#!/usr/bin/env bash".to_string(),
        "set -euo pipefail".to_string(),
        format!("mkdir -p {}", sh_quote(&output_root_string)),
        "ffmpeg -y \\".to_string(),
        format!("  -i {} \\", sh_quote(&input_path.to_string_lossy())),
        format!("  -filter_complex {} \\", sh_quote(&filter_complex)),
    ];

    for (index, variant) in form.variants.iter().enumerate() {
        lines.push(format!("  -map [v{index}out] -map a:0 \\"));
        lines.push(format!(
            "  -c:v:{index} {video_encoder} -profile:v:{index} {profile} \\"
        ));

        if let Some((argument_name, argument_value)) = preset_argument.as_ref() {
            lines.push(format!("  -{argument_name}:v:{index} {argument_value} \\"));
        }

        lines.push(format!(
            "  -b:v:{index} {}k -maxrate:v:{index} {}k -bufsize:v:{index} {}k \\",
            variant.video_bitrate_k, variant.maxrate_k, variant.bufsize_k
        ));
        lines.push(format!(
            "  -g:v:{index} {} -keyint_min:v:{index} {} \\",
            form.gop, form.gop
        ));

        if matches!(
            form.video_codec_lib,
            VideoCodecLib::X264 | VideoCodecLib::X265
        ) {
            lines.push(format!(
                "  -sc_threshold:v:{index} {} \\",
                form.sc_threshold
            ));
        }

        lines.push(format!(
            "  -c:a:{index} {audio_encoder} -b:a:{index} {}k -ac:a:{index} {} \\",
            variant.audio_bitrate_k, form.audio_channels
        ));
    }

    lines.push("  -pix_fmt yuv420p \\".to_string());
    lines.push("  -f hls \\".to_string());
    lines.push(format!("  -hls_time {} \\", form.hls_time_seconds));

    if let Some(playlist_type_value) = ffmpeg_playlist_type(form.hls_playlist_type) {
        lines.push(format!("  -hls_playlist_type {playlist_type_value} \\"));
    }

    if form.hls_flags_independent_segments {
        lines.push("  -hls_flags independent_segments \\".to_string());
    }

    lines.push(format!(
        "  -hls_segment_filename {} \\",
        sh_quote(&segment_filename.to_string_lossy())
    ));
    lines.push(format!(
        "  -master_pl_name {} \\",
        sh_quote(&master_playlist_name)
    ));
    lines.push(format!(
        "  -var_stream_map {} \\",
        sh_quote(&var_stream_map)
    ));
    lines.push(format!("{}", sh_quote(&variant_playlist.to_string_lossy())));

    lines.join("\n")
}

fn ffmpeg_script_popup_markdown(script: &str) -> String {
    format!("```bash\n{script}\n```")
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

fn output_root_folder(form: &EncodeOptionsForm) -> PathBuf {
    let base_folder = non_empty(&form.output_base_folder).unwrap_or_else(|| ".".to_string());
    let mut output_root = PathBuf::from(base_folder);

    if let Some(subfolder_name) = non_empty(&form.output_subfolder_name) {
        output_root.push(subfolder_name);
    }

    output_root
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn video_encoder_name(codec: VideoCodecLib) -> &'static str {
    match codec {
        VideoCodecLib::X264 => "libx264",
        VideoCodecLib::X265 => "libx265",
        VideoCodecLib::Vp9 => "libvpx-vp9",
        VideoCodecLib::Av1 => "libaom-av1",
    }
}

fn audio_encoder_name(codec: AudioCodec) -> &'static str {
    match codec {
        AudioCodec::Aac => "aac",
        AudioCodec::Opus => "libopus",
        AudioCodec::Mp3 => "libmp3lame",
    }
}

fn ffmpeg_profile_value(codec: VideoCodecLib, profile: VideoProfile) -> String {
    match codec {
        VideoCodecLib::Vp9 => match profile {
            VideoProfile::Profile0 => "0".to_string(),
            VideoProfile::Profile1 => "1".to_string(),
            VideoProfile::Profile2 => "2".to_string(),
            VideoProfile::Profile3 => "3".to_string(),
            _ => "0".to_string(),
        },
        _ => profile.to_string(),
    }
}

fn codec_preset_argument(codec: VideoCodecLib, preset: X264Preset) -> Option<(String, String)> {
    let value = preset.to_string();

    match codec {
        VideoCodecLib::X264 | VideoCodecLib::X265 => Some(("preset".to_string(), value)),
        VideoCodecLib::Vp9 => Some(("deadline".to_string(), value)),
        VideoCodecLib::Av1 => {
            let cpu_used = match preset {
                X264Preset::Fast => "8",
                X264Preset::Medium => "5",
                X264Preset::Slow => "3",
                _ => "5",
            };
            Some(("cpu-used".to_string(), cpu_used.to_string()))
        }
    }
}

fn ffmpeg_playlist_type(value: HlsPlaylistType) -> Option<&'static str> {
    match value {
        HlsPlaylistType::Vod => Some("vod"),
        HlsPlaylistType::Event => Some("event"),
        HlsPlaylistType::Live => None,
    }
}

fn sanitize_variant_name(value: &str, index: usize) -> String {
    let candidate = value.trim();
    let source = if candidate.is_empty() {
        format!("variant{}", index + 1)
    } else {
        candidate.to_string()
    };

    source
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
