use crate::app::{
    AppState, AudioCodec, EncodeOptionsForm, EncodeProgress, EncodeRuntimeState, EncodeStatus,
    EncodeWorkerEvent, HLSenpai, HlsPlaylistType, VideoCodecLib, VideoProfile, X264Preset,
};
use crate::ff_helpers::{
    PreviewVideo, extract_video_metadata, validate_video_file, video_metadata_markdown_sections,
};
use iced::Task;
use iced::widget::markdown;
use iced_video_player::Video;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
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
    EncodePressed,
    EncodeCancelPressed,
    EncodePollTick,
    EncodeLogModalOpen,
    EncodeLogModalClose,
    PrintFfmpegScript,
    CloseFfmpegScriptPopup,
}

pub(crate) fn handle_messages(app: &mut HLSenpai, message: Message) -> Task<Message> {
    match message {
        Message::SelectFilePressed => {
            app.ffmpeg_script_popup = None;
            app.encode_runtime = None;
            app.show_encode_log_modal = false;

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
                        let metadata_markdown_sections = video_metadata_markdown_sections(&metadata)
                            .into_iter()
                            .map(|section| markdown::Content::parse(&section))
                            .collect::<Vec<_>>();

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
                                    metadata_markdown_sections,
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
            if app
                .encode_runtime
                .as_ref()
                .is_some_and(EncodeRuntimeState::is_running)
            {
                eprintln!("Cannot navigate back while encoding is active.");
                return Task::none();
            }

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
        Message::EncodePressed => {
            if app
                .encode_runtime
                .as_ref()
                .is_some_and(EncodeRuntimeState::is_running)
            {
                return Task::none();
            }

            if let (Some(video), Some(form)) = (app.video.as_ref(), app.encode_options.as_ref()) {
                let (sender, receiver) = mpsc::channel();
                let cancel_flag = Arc::new(AtomicBool::new(false));
                let duration_ms = seconds_to_ffmpeg_progress_units(video.metadata.duration_seconds);
                let has_audio = video.metadata.audio_codec.is_some();

                app.encode_runtime = Some(EncodeRuntimeState::new(
                    receiver,
                    Arc::clone(&cancel_flag),
                    duration_ms,
                ));
                app.show_encode_log_modal = true;

                start_encode_worker(
                    video._path.clone(),
                    form.clone(),
                    has_audio,
                    sender,
                    Arc::clone(&cancel_flag),
                );
            }
        }
        Message::EncodeCancelPressed => {
            if let Some(runtime) = app.encode_runtime.as_mut()
                && runtime.can_cancel()
            {
                runtime.status = EncodeStatus::Canceling;
                runtime.cancel_flag.store(true, Ordering::Relaxed);
                runtime.append_log_line("Cancellation requested...".to_string());
            }
        }
        Message::EncodePollTick => {
            if let Some(runtime) = app.encode_runtime.as_mut() {
                let mut disconnected = false;
                let mut log_lines_appended = false;

                loop {
                    match runtime.receiver.try_recv() {
                        Ok(event) => {
                            log_lines_appended |= apply_encode_worker_event(runtime, event);
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }

                if disconnected && runtime.is_running() {
                    runtime.status = EncodeStatus::Failed;
                    runtime
                        .append_log_line("Encoding worker disconnected unexpectedly.".to_string());
                    log_lines_appended = true;
                }

                if log_lines_appended && app.show_encode_log_modal {
                    return iced::widget::operation::snap_to_end("encode-log-scroll");
                }
            }
        }
        Message::EncodeLogModalOpen => {
            app.show_encode_log_modal = true;
            return iced::widget::operation::snap_to_end("encode-log-scroll");
        }
        Message::EncodeLogModalClose => {
            app.show_encode_log_modal = false;
        }
        Message::PrintFfmpegScript => {
            if let (Some(video), Some(form)) = (app.video.as_ref(), app.encode_options.as_ref()) {
                let script =
                    build_ffmpeg_script(&video._path, form, video.metadata.audio_codec.is_some());
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

fn apply_encode_worker_event(runtime: &mut EncodeRuntimeState, event: EncodeWorkerEvent) -> bool {
    let mut log_changed = false;

    match event {
        EncodeWorkerEvent::Started => {
            runtime.append_log_line("ffmpeg process started.".to_string());
            log_changed = true;
        }
        EncodeWorkerEvent::LogLine(line) => {
            runtime.append_log_line(line);
            log_changed = true;
        }
        EncodeWorkerEvent::Progress(progress) => {
            if let Some(out_time_ms) = progress.out_time_ms {
                runtime.last_out_time_ms = Some(out_time_ms);

                if let Some(total_ms) = runtime.duration_ms
                    && total_ms > 0
                {
                    let percent = (out_time_ms as f64 / total_ms as f64 * 100.0).clamp(0.0, 100.0);
                    runtime.progress_percent = Some(percent as f32);
                }
            }

            if let Some(speed) = progress.speed {
                runtime.speed = Some(speed);
            }

            if let Some(bitrate) = progress.bitrate {
                runtime.bitrate = Some(bitrate);
            }

            if progress.progress_marker.as_deref() == Some("end") {
                runtime.progress_percent = Some(100.0);
            }
        }
        EncodeWorkerEvent::Finished {
            exit_code,
            was_canceled,
        } => {
            if was_canceled {
                runtime.status = EncodeStatus::Canceled;
                runtime.append_log_line("Encode canceled.".to_string());
                log_changed = true;
            } else if exit_code == Some(0) {
                runtime.status = EncodeStatus::Success;
                runtime.progress_percent = Some(100.0);
                runtime.append_log_line("Encode completed successfully.".to_string());
                log_changed = true;
            } else {
                runtime.status = EncodeStatus::Failed;
                runtime.append_log_line(format!(
                    "Encode failed with exit code: {}",
                    exit_code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
                log_changed = true;
            }
        }
        EncodeWorkerEvent::SpawnError(message) => {
            runtime.status = EncodeStatus::Failed;
            runtime.append_log_line(format!("Could not start ffmpeg: {message}"));
            log_changed = true;
        }
    }

    log_changed
}

fn start_encode_worker(
    input_path: PathBuf,
    form: EncodeOptionsForm,
    has_audio: bool,
    sender: mpsc::Sender<EncodeWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    thread::spawn(move || run_encode_worker(input_path, form, has_audio, sender, cancel_flag));
}

fn run_encode_worker(
    input_path: PathBuf,
    form: EncodeOptionsForm,
    has_audio: bool,
    sender: mpsc::Sender<EncodeWorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    let (output_root, args) = build_ffmpeg_args(&input_path, &form, has_audio);

    if let Err(err) = fs::create_dir_all(&output_root) {
        let _ = sender.send(EncodeWorkerEvent::SpawnError(format!(
            "Could not create output folder {}: {err}",
            output_root.display()
        )));
        return;
    }

    let mut command = Command::new("ffmpeg");
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = sender.send(EncodeWorkerEvent::SpawnError(err.to_string()));
            return;
        }
    };

    let _ = sender.send(EncodeWorkerEvent::Started);
    if !has_audio {
        let _ = sender.send(EncodeWorkerEvent::LogLine(
            "Input has no audio stream. Encoding video-only renditions.".to_string(),
        ));
    }

    let stdout_handle = child.stdout.take().map(|stdout| {
        let tx = sender.clone();
        thread::spawn(move || read_progress_output(stdout, tx))
    });

    let stderr_handle = child.stderr.take().map(|stderr| {
        let tx = sender.clone();
        thread::spawn(move || read_stderr_output(stderr, tx))
    });

    let mut was_canceled = false;
    let mut cancel_sent = false;

    let exit_code = loop {
        if cancel_flag.load(Ordering::Relaxed) && !cancel_sent {
            cancel_sent = true;
            was_canceled = true;

            if let Err(err) = child.kill()
                && err.kind() != ErrorKind::InvalidInput
            {
                let _ = sender.send(EncodeWorkerEvent::LogLine(format!(
                    "Failed to terminate ffmpeg process: {err}"
                )));
            }
        }

        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                thread::sleep(Duration::from_millis(120));
            }
            Err(err) => {
                let _ = sender.send(EncodeWorkerEvent::LogLine(format!(
                    "Error while waiting for ffmpeg process: {err}"
                )));
                break None;
            }
        }
    };

    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    let _ = sender.send(EncodeWorkerEvent::Finished {
        exit_code,
        was_canceled,
    });
}

fn read_progress_output(stdout: ChildStdout, sender: mpsc::Sender<EncodeWorkerEvent>) {
    let reader = BufReader::new(stdout);
    let mut snapshot = EncodeProgress::default();

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(line) => line,
            Err(err) => {
                let _ = sender.send(EncodeWorkerEvent::LogLine(format!(
                    "Error reading ffmpeg progress: {err}"
                )));
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let _ = sender.send(EncodeWorkerEvent::LogLine(format!("[progress] {trimmed}")));

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        match key {
            "out_time_ms" => {
                if let Ok(parsed) = value.parse::<u64>() {
                    snapshot.out_time_ms = Some(parsed);
                }
            }
            "speed" => {
                snapshot.speed = Some(value.to_string());
            }
            "bitrate" => {
                snapshot.bitrate = Some(value.to_string());
            }
            "progress" => {
                snapshot.progress_marker = Some(value.to_string());
                let _ = sender.send(EncodeWorkerEvent::Progress(snapshot.clone()));
            }
            _ => {}
        }
    }
}

fn read_stderr_output(stderr: ChildStderr, sender: mpsc::Sender<EncodeWorkerEvent>) {
    let reader = BufReader::new(stderr);

    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if !line.trim().is_empty() {
                    let _ = sender.send(EncodeWorkerEvent::LogLine(line));
                }
            }
            Err(err) => {
                let _ = sender.send(EncodeWorkerEvent::LogLine(format!(
                    "Error reading ffmpeg stderr: {err}"
                )));
                break;
            }
        }
    }
}

fn build_ffmpeg_args(
    input_path: &Path,
    form: &EncodeOptionsForm,
    has_audio: bool,
) -> (PathBuf, Vec<String>) {
    let output_root = output_root_folder(form);
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
            if has_audio {
                format!(
                    "v:{index},a:{index},name:{}",
                    sanitize_variant_name(&variant.name, index)
                )
            } else {
                format!(
                    "v:{index},name:{}",
                    sanitize_variant_name(&variant.name, index)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string_lossy().to_string(),
        "-filter_complex".to_string(),
        filter_complex,
    ];

    for (index, variant) in form.variants.iter().enumerate() {
        args.push("-map".to_string());
        args.push(format!("[v{index}out]"));
        if has_audio {
            args.push("-map".to_string());
            args.push("a:0".to_string());
        }

        args.push(format!("-c:v:{index}"));
        args.push(video_encoder.to_string());
        args.push(format!("-profile:v:{index}"));
        args.push(profile.clone());

        if let Some((argument_name, argument_value)) = preset_argument.as_ref() {
            args.push(format!("-{argument_name}:v:{index}"));
            args.push(argument_value.clone());
        }

        args.push(format!("-b:v:{index}"));
        args.push(format!("{}k", variant.video_bitrate_k));
        args.push(format!("-maxrate:v:{index}"));
        args.push(format!("{}k", variant.maxrate_k));
        args.push(format!("-bufsize:v:{index}"));
        args.push(format!("{}k", variant.bufsize_k));

        args.push(format!("-g:v:{index}"));
        args.push(form.gop.to_string());
        args.push(format!("-keyint_min:v:{index}"));
        args.push(form.gop.to_string());

        if matches!(
            form.video_codec_lib,
            VideoCodecLib::X264 | VideoCodecLib::X265
        ) {
            args.push(format!("-sc_threshold:v:{index}"));
            args.push(form.sc_threshold.to_string());
        }

        if has_audio {
            args.push(format!("-c:a:{index}"));
            args.push(audio_encoder.to_string());
            args.push(format!("-b:a:{index}"));
            args.push(format!("{}k", variant.audio_bitrate_k));
            args.push(format!("-ac:a:{index}"));
            args.push(form.audio_channels.to_string());
        }
    }

    args.push("-pix_fmt".to_string());
    args.push("yuv420p".to_string());
    args.push("-f".to_string());
    args.push("hls".to_string());
    args.push("-hls_time".to_string());
    args.push(form.hls_time_seconds.to_string());

    if let Some(playlist_type_value) = ffmpeg_playlist_type(form.hls_playlist_type) {
        args.push("-hls_playlist_type".to_string());
        args.push(playlist_type_value.to_string());
    }

    if form.hls_flags_independent_segments {
        args.push("-hls_flags".to_string());
        args.push("independent_segments".to_string());
    }

    args.push("-hls_segment_filename".to_string());
    args.push(segment_filename.to_string_lossy().to_string());
    args.push("-master_pl_name".to_string());
    args.push(master_playlist_name);
    args.push("-var_stream_map".to_string());
    args.push(var_stream_map);
    args.push("-progress".to_string());
    args.push("pipe:1".to_string());
    args.push("-nostats".to_string());
    args.push(variant_playlist.to_string_lossy().to_string());

    (output_root, args)
}

fn build_ffmpeg_script(input_path: &Path, form: &EncodeOptionsForm, has_audio: bool) -> String {
    let (output_root, args) = build_ffmpeg_args(input_path, form, has_audio);
    let output_root_string = output_root.to_string_lossy().to_string();

    let mut lines = vec![
        "#!/usr/bin/env bash".to_string(),
        "set -euo pipefail".to_string(),
        format!("mkdir -p {}", sh_quote(&output_root_string)),
        "ffmpeg \\".to_string(),
    ];

    for (index, argument) in args.iter().enumerate() {
        let suffix = if index + 1 == args.len() { "" } else { " \\" };
        lines.push(format!("  {}{suffix}", sh_quote(argument)));
    }

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

fn seconds_to_ffmpeg_progress_units(value: Option<f64>) -> Option<u64> {
    let seconds = value?;
    if seconds.is_finite() && seconds > 0.0 {
        // ffmpeg `-progress` reports `out_time_ms` in microseconds despite the field name.
        Some((seconds * 1_000_000.0) as u64)
    } else {
        None
    }
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
