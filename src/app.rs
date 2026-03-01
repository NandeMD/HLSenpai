use crate::config::{
    AppAuthConfig, ConfigPaths, PersistedEncodePreset, PersistedUploadPrefs, load_auth_config,
    load_last_preset, load_upload_prefs, save_auth_config, save_last_preset, save_upload_prefs,
};
use crate::ff_helpers::{PreviewVideo, VideoMetadata};
use crate::message::*;
use crate::upload::{UploadOverwriteMode, UploadProgress, UploadTargetKind, UploadWorkerEvent};
use crate::views;

use iced::widget::markdown;
use iced::{Element, Subscription, Task, Theme};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) enum AppState {
    #[default]
    Initial,
    VideoOverview,
    EncodeOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VideoProfile {
    Baseline,
    Main,
    #[default]
    High,
    Main10,
    MainStillPicture,
    Profile0,
    Profile1,
    Profile2,
    Profile3,
    Professional,
}

impl VideoProfile {
    pub const X264: [Self; 3] = [Self::Baseline, Self::Main, Self::High];
    pub const X265: [Self; 3] = [Self::Main, Self::Main10, Self::MainStillPicture];
    pub const VP9: [Self; 4] = [
        Self::Profile0,
        Self::Profile1,
        Self::Profile2,
        Self::Profile3,
    ];
    pub const AV1: [Self; 3] = [Self::Main, Self::High, Self::Professional];
}

impl fmt::Display for VideoProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Baseline => write!(f, "baseline"),
            Self::Main => write!(f, "main"),
            Self::High => write!(f, "high"),
            Self::Main10 => write!(f, "main10"),
            Self::MainStillPicture => write!(f, "mainstillpicture"),
            Self::Profile0 => write!(f, "profile0"),
            Self::Profile1 => write!(f, "profile1"),
            Self::Profile2 => write!(f, "profile2"),
            Self::Profile3 => write!(f, "profile3"),
            Self::Professional => write!(f, "professional"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VideoCodecLib {
    #[default]
    X264,
    X265,
    Vp9,
    Av1,
}

impl VideoCodecLib {
    pub const ALL: [Self; 4] = [Self::X264, Self::X265, Self::Vp9, Self::Av1];

    pub fn profile_options(self) -> &'static [VideoProfile] {
        match self {
            Self::X264 => &VideoProfile::X264,
            Self::X265 => &VideoProfile::X265,
            Self::Vp9 => &VideoProfile::VP9,
            Self::Av1 => &VideoProfile::AV1,
        }
    }

    pub fn preset_options(self) -> &'static [X264Preset] {
        match self {
            Self::X264 | Self::X265 => &X264Preset::X26X,
            Self::Vp9 => &X264Preset::VP9,
            Self::Av1 => &X264Preset::AV1,
        }
    }

    pub fn default_profile(self) -> VideoProfile {
        match self {
            Self::X264 => VideoProfile::High,
            Self::X265 => VideoProfile::Main,
            Self::Vp9 => VideoProfile::Profile0,
            Self::Av1 => VideoProfile::Main,
        }
    }

    pub fn default_preset(self) -> X264Preset {
        match self {
            Self::X264 => X264Preset::Veryfast,
            Self::X265 => X264Preset::Medium,
            Self::Vp9 => X264Preset::Good,
            Self::Av1 => X264Preset::Medium,
        }
    }

    pub fn default_sc_threshold(self) -> i32 {
        match self {
            Self::X264 | Self::X265 => 0,
            Self::Vp9 | Self::Av1 => 40,
        }
    }
}

impl fmt::Display for VideoCodecLib {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X264 => write!(f, "x264"),
            Self::X265 => write!(f, "x265"),
            Self::Vp9 => write!(f, "vp9"),
            Self::Av1 => write!(f, "av1"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum X264Preset {
    Ultrafast,
    Superfast,
    #[default]
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
    Realtime,
    Good,
    Best,
}

impl X264Preset {
    pub const X26X: [Self; 9] = [
        Self::Ultrafast,
        Self::Superfast,
        Self::Veryfast,
        Self::Faster,
        Self::Fast,
        Self::Medium,
        Self::Slow,
        Self::Slower,
        Self::Veryslow,
    ];
    pub const VP9: [Self; 3] = [Self::Realtime, Self::Good, Self::Best];
    pub const AV1: [Self; 3] = [Self::Fast, Self::Medium, Self::Slow];
}

impl fmt::Display for X264Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ultrafast => write!(f, "ultrafast"),
            Self::Superfast => write!(f, "superfast"),
            Self::Veryfast => write!(f, "veryfast"),
            Self::Faster => write!(f, "faster"),
            Self::Fast => write!(f, "fast"),
            Self::Medium => write!(f, "medium"),
            Self::Slow => write!(f, "slow"),
            Self::Slower => write!(f, "slower"),
            Self::Veryslow => write!(f, "veryslow"),
            Self::Realtime => write!(f, "realtime"),
            Self::Good => write!(f, "good"),
            Self::Best => write!(f, "best"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AudioCodec {
    #[default]
    Aac,
    Opus,
    Mp3,
}

impl AudioCodec {
    pub const ALL: [Self; 3] = [Self::Aac, Self::Opus, Self::Mp3];
}

impl fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aac => write!(f, "aac"),
            Self::Opus => write!(f, "opus"),
            Self::Mp3 => write!(f, "mp3"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HlsPlaylistType {
    #[default]
    Vod,
    Event,
    Live,
}

impl HlsPlaylistType {
    pub const ALL: [Self; 3] = [Self::Vod, Self::Event, Self::Live];
}

impl fmt::Display for HlsPlaylistType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vod => write!(f, "vod"),
            Self::Event => write!(f, "event"),
            Self::Live => write!(f, "live"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VariantForm {
    pub name: String,
    pub video_bitrate_k: u32,
    pub maxrate_k: u32,
    pub bufsize_k: u32,
    pub audio_bitrate_k: u32,
}

impl VariantForm {
    pub(crate) fn defaults() -> [Self; 3] {
        [
            Self {
                name: "low".to_string(),
                video_bitrate_k: 1200,
                maxrate_k: 1500,
                bufsize_k: 2400,
                audio_bitrate_k: 96,
            },
            Self {
                name: "med".to_string(),
                video_bitrate_k: 2500,
                maxrate_k: 3000,
                bufsize_k: 5000,
                audio_bitrate_k: 128,
            },
            Self {
                name: "high".to_string(),
                video_bitrate_k: 4500,
                maxrate_k: 5400,
                bufsize_k: 9000,
                audio_bitrate_k: 128,
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EncodeOptionsForm {
    pub scale_width: u32,
    pub scale_height: u32,
    pub scale_lock_aspect: bool,
    pub source_width: u32,
    pub source_height: u32,
    pub source_aspect_num: u32,
    pub source_aspect_den: u32,
    pub gop: u32,
    pub video_codec_lib: VideoCodecLib,
    pub profile: VideoProfile,
    pub preset: X264Preset,
    pub sc_threshold: i32,
    pub audio_codec: AudioCodec,
    pub audio_channels: u8,
    pub hls_time_seconds: u32,
    pub hls_playlist_type: HlsPlaylistType,
    pub hls_flags_independent_segments: bool,
    pub master_playlist_name: String,
    pub segment_filename_pattern: String,
    pub output_variant_playlist_pattern: String,
    pub output_base_folder: String,
    pub output_subfolder_name: String,
    pub output_master_playlist_file: String,
    pub variants: [VariantForm; 3],
}

impl EncodeOptionsForm {
    pub(crate) fn from_metadata(metadata: &VideoMetadata) -> Self {
        let (source_width, source_height) =
            parse_resolution(metadata.video_resolution.as_deref()).unwrap_or((1280, 720));
        let (source_aspect_num, source_aspect_den) = reduce_ratio(source_width, source_height);

        let gop = metadata
            .framerate
            .filter(|fps| *fps > 0.0)
            .map(|fps| (fps * 2.0).round() as u32)
            .map(|value| value.max(1))
            .unwrap_or(60);

        let video_codec_lib = VideoCodecLib::default();

        Self {
            scale_width: source_width,
            scale_height: source_height,
            scale_lock_aspect: true,
            source_width,
            source_height,
            source_aspect_num,
            source_aspect_den,
            gop,
            video_codec_lib,
            profile: video_codec_lib.default_profile(),
            preset: video_codec_lib.default_preset(),
            sc_threshold: video_codec_lib.default_sc_threshold(),
            audio_codec: AudioCodec::default(),
            audio_channels: 2,
            hls_time_seconds: 6,
            hls_playlist_type: HlsPlaylistType::default(),
            hls_flags_independent_segments: true,
            master_playlist_name: "master.m3u8".to_string(),
            segment_filename_pattern: "v%v/seg_%06d.ts".to_string(),
            output_variant_playlist_pattern: "v%v/prog.m3u8".to_string(),
            output_base_folder: "out".to_string(),
            output_subfolder_name: "myvideo123".to_string(),
            output_master_playlist_file: "master.m3u8".to_string(),
            variants: VariantForm::defaults(),
        }
    }

    pub(crate) fn source_aspect_label(&self) -> String {
        format!("{}:{}", self.source_aspect_num, self.source_aspect_den)
    }

    pub(crate) fn source_resolution_label(&self) -> String {
        format!("{}x{}", self.source_width, self.source_height)
    }

    pub(crate) fn set_scale_width(&mut self, width: u32) {
        self.scale_width = width.max(1);

        if self.scale_lock_aspect {
            self.scale_height = calculate_height_for_width(
                self.scale_width,
                self.source_aspect_num,
                self.source_aspect_den,
            );
        }
    }

    pub(crate) fn set_scale_height(&mut self, height: u32) {
        self.scale_height = height.max(1);

        if self.scale_lock_aspect {
            self.scale_width = calculate_width_for_height(
                self.scale_height,
                self.source_aspect_num,
                self.source_aspect_den,
            );
        }
    }

    pub(crate) fn set_scale_lock_aspect(&mut self, value: bool) {
        self.scale_lock_aspect = value;
    }

    pub(crate) fn profile_options(&self) -> &'static [VideoProfile] {
        self.video_codec_lib.profile_options()
    }

    pub(crate) fn preset_options(&self) -> &'static [X264Preset] {
        self.video_codec_lib.preset_options()
    }

    pub(crate) fn apply_codec_defaults(&mut self, codec: VideoCodecLib) {
        self.video_codec_lib = codec;
        self.profile = codec.default_profile();
        self.preset = codec.default_preset();
        self.sc_threshold = codec.default_sc_threshold();
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct EncodeProgress {
    pub out_time_ms: Option<u64>,
    pub speed: Option<String>,
    pub bitrate: Option<String>,
    pub progress_marker: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum EncodeWorkerEvent {
    Started,
    LogLine(String),
    Progress(EncodeProgress),
    Finished {
        exit_code: Option<i32>,
        was_canceled: bool,
        output_root: PathBuf,
    },
    SpawnError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EncodeStatus {
    Running,
    Canceling,
    Success,
    Failed,
    Canceled,
}

pub(crate) struct EncodeRuntimeState {
    pub status: EncodeStatus,
    pub started_at: Instant,
    pub progress_percent: Option<f32>,
    pub last_out_time_ms: Option<u64>,
    pub speed: Option<String>,
    pub bitrate: Option<String>,
    pub log_lines: Vec<String>,
    pub receiver: Receiver<EncodeWorkerEvent>,
    pub cancel_flag: Arc<AtomicBool>,
    pub duration_ms: Option<u64>,
    pub output_root: Option<PathBuf>,
}

impl EncodeRuntimeState {
    const MAX_LOG_LINES: usize = 2_000;

    pub(crate) fn new(
        receiver: Receiver<EncodeWorkerEvent>,
        cancel_flag: Arc<AtomicBool>,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            status: EncodeStatus::Running,
            started_at: Instant::now(),
            progress_percent: None,
            last_out_time_ms: None,
            speed: None,
            bitrate: None,
            log_lines: Vec::new(),
            receiver,
            cancel_flag,
            duration_ms,
            output_root: None,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.status, EncodeStatus::Running | EncodeStatus::Canceling)
    }

    pub(crate) fn can_cancel(&self) -> bool {
        matches!(self.status, EncodeStatus::Running | EncodeStatus::Canceling)
    }

    pub(crate) fn append_log_line(&mut self, line: String) {
        self.log_lines.push(line);
        let overflow = self.log_lines.len().saturating_sub(Self::MAX_LOG_LINES);
        if overflow > 0 {
            self.log_lines.drain(0..overflow);
        }
    }

    pub(crate) fn status_label(&self) -> &'static str {
        match self.status {
            EncodeStatus::Running => "Running",
            EncodeStatus::Canceling => "Canceling",
            EncodeStatus::Success => "Success",
            EncodeStatus::Failed => "Failed",
            EncodeStatus::Canceled => "Canceled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UploadStatus {
    Running,
    Canceling,
    Success,
    Failed,
    Canceled,
}

pub(crate) struct UploadRuntimeState {
    pub status: UploadStatus,
    pub started_at: Instant,
    pub progress_percent: Option<f32>,
    pub uploaded_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub total_files: usize,
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub log_lines: Vec<String>,
    pub receiver: Receiver<UploadWorkerEvent>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl UploadRuntimeState {
    const MAX_LOG_LINES: usize = 2_000;

    pub(crate) fn new(receiver: Receiver<UploadWorkerEvent>, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            status: UploadStatus::Running,
            started_at: Instant::now(),
            progress_percent: None,
            uploaded_files: 0,
            skipped_files: 0,
            failed_files: 0,
            total_files: 0,
            uploaded_bytes: 0,
            total_bytes: 0,
            log_lines: Vec::new(),
            receiver,
            cancel_flag,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.status, UploadStatus::Running | UploadStatus::Canceling)
    }

    pub(crate) fn can_cancel(&self) -> bool {
        matches!(self.status, UploadStatus::Running | UploadStatus::Canceling)
    }

    pub(crate) fn append_log_line(&mut self, line: String) {
        self.log_lines.push(line);
        let overflow = self.log_lines.len().saturating_sub(Self::MAX_LOG_LINES);
        if overflow > 0 {
            self.log_lines.drain(0..overflow);
        }
    }

    pub(crate) fn apply_progress(&mut self, progress: &UploadProgress) {
        self.progress_percent = Some(progress.percent.clamp(0.0, 100.0));
        self.uploaded_files = progress.uploaded_files;
        self.skipped_files = progress.skipped_files;
        self.failed_files = progress.failed_files;
        self.total_files = progress.total_files;
        self.uploaded_bytes = progress.uploaded_bytes;
        self.total_bytes = progress.total_bytes;
    }

    pub(crate) fn status_label(&self) -> &'static str {
        match self.status {
            UploadStatus::Running => "Running",
            UploadStatus::Canceling => "Canceling",
            UploadStatus::Success => "Success",
            UploadStatus::Failed => "Failed",
            UploadStatus::Canceled => "Canceled",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UploadFormState {
    pub available_targets: Vec<UploadTargetKind>,
    pub selected_target: Option<UploadTargetKind>,
    pub bucket: String,
    pub prefix: String,
    pub overwrite_mode: UploadOverwriteMode,
}

impl UploadFormState {
    pub(crate) fn is_ready(&self) -> bool {
        self.selected_target.is_some() && !self.bucket.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AwsCredentialsFormState {
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
}

pub(crate) struct HLSenpai {
    pub video: Option<PreviewVideo>,
    pub state: AppState,
    pub encode_options: Option<EncodeOptionsForm>,
    pub ffmpeg_script_popup: Option<markdown::Content>,
    pub encode_runtime: Option<EncodeRuntimeState>,
    pub show_encode_log_modal: bool,
    pub show_upload_modal: bool,
    pub show_upload_credentials_modal: bool,
    pub upload_runtime: Option<UploadRuntimeState>,
    pub upload_form: UploadFormState,
    pub upload_credentials_form: AwsCredentialsFormState,
    pub last_encode_output_root: Option<PathBuf>,
    pub config_paths: ConfigPaths,
    pub auth_config: AppAuthConfig,
    pub last_preset: Option<PersistedEncodePreset>,
    pub upload_prefs: PersistedUploadPrefs,
}

impl Default for HLSenpai {
    fn default() -> Self {
        Self::new()
    }
}

impl HLSenpai {
    pub(crate) fn new() -> Self {
        let config_paths = ConfigPaths::discover().unwrap_or_else(|err| {
            eprintln!(
                "Could not resolve OS config directory ({}). Falling back to current directory.",
                err
            );
            ConfigPaths::fallback_current_dir()
        });

        let auth_config = match load_auth_config(&config_paths) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Could not load auth config: {}. Using defaults.", err);
                AppAuthConfig::default()
            }
        };

        if let Err(err) = save_auth_config(&config_paths, &auth_config) {
            eprintln!("Could not persist auth config bootstrap file: {}", err);
        }

        let last_preset = match load_last_preset(&config_paths) {
            Ok(preset) => preset,
            Err(err) => {
                eprintln!("Could not load last encoding preset: {}. Ignoring.", err);
                None
            }
        };

        let upload_prefs = match load_upload_prefs(&config_paths) {
            Ok(prefs) => prefs,
            Err(err) => {
                eprintln!(
                    "Could not load upload preferences: {}. Using defaults.",
                    err
                );
                PersistedUploadPrefs::default()
            }
        };

        Self {
            video: None,
            state: AppState::Initial,
            encode_options: None,
            ffmpeg_script_popup: None,
            encode_runtime: None,
            show_encode_log_modal: false,
            show_upload_modal: false,
            show_upload_credentials_modal: false,
            upload_runtime: None,
            upload_form: UploadFormState {
                overwrite_mode: upload_prefs
                    .last_overwrite_mode
                    .as_deref()
                    .map(UploadOverwriteMode::from_str)
                    .unwrap_or_default(),
                bucket: upload_prefs.last_bucket.clone().unwrap_or_default(),
                prefix: upload_prefs.last_prefix.clone().unwrap_or_default(),
                selected_target: upload_prefs
                    .last_upload_provider
                    .as_deref()
                    .and_then(|value| {
                        if value == "aws_s3" {
                            Some(UploadTargetKind::AwsS3)
                        } else {
                            None
                        }
                    }),
                ..UploadFormState::default()
            },
            upload_credentials_form: AwsCredentialsFormState::default(),
            last_encode_output_root: None,
            config_paths,
            auth_config,
            last_preset,
            upload_prefs,
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        handle_messages(self, message)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let encode_subscription = if self
            .encode_runtime
            .as_ref()
            .is_some_and(EncodeRuntimeState::is_running)
        {
            Some(iced::time::every(Duration::from_millis(200)).map(|_| Message::EncodePollTick))
        } else {
            None
        };

        let upload_subscription = if self
            .upload_runtime
            .as_ref()
            .is_some_and(UploadRuntimeState::is_running)
        {
            Some(iced::time::every(Duration::from_millis(200)).map(|_| Message::UploadPollTick))
        } else {
            None
        };

        match (encode_subscription, upload_subscription) {
            (Some(encode), Some(upload)) => Subscription::batch(vec![encode, upload]),
            (Some(encode), None) => encode,
            (None, Some(upload)) => upload,
            (None, None) => Subscription::none(),
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        match self.state {
            AppState::Initial => views::select_file(self),
            AppState::VideoOverview => views::video_overview(self),
            AppState::EncodeOptions => views::encode_options(self),
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    pub(crate) fn persist_last_preset(&mut self) {
        let Some(form) = self.encode_options.as_ref() else {
            return;
        };

        let preset = PersistedEncodePreset::from_form(form);
        match save_last_preset(&self.config_paths, &preset) {
            Ok(()) => {
                self.last_preset = Some(preset);
            }
            Err(err) => {
                eprintln!("Could not save last encoding preset: {}", err);
            }
        }
    }

    pub(crate) fn persist_upload_prefs(&mut self) {
        self.upload_prefs.last_upload_provider =
            self.upload_form.selected_target.map(|target| match target {
                UploadTargetKind::AwsS3 => "aws_s3".to_string(),
            });
        self.upload_prefs.last_overwrite_mode =
            Some(self.upload_form.overwrite_mode.as_str().to_string());
        self.upload_prefs.last_bucket = non_empty(&self.upload_form.bucket);
        self.upload_prefs.last_prefix = non_empty(&self.upload_form.prefix);

        if let Err(err) = save_upload_prefs(&self.config_paths, &self.upload_prefs) {
            eprintln!("Could not save upload preferences: {}", err);
        }
    }
}

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn parse_resolution(value: Option<&str>) -> Option<(u32, u32)> {
    let source = value?.trim();
    let (width, height) = source.split_once('x')?;
    let width = width.trim().parse::<u32>().ok()?;
    let height = height.trim().parse::<u32>().ok()?;

    if width == 0 || height == 0 {
        return None;
    }

    Some((width, height))
}

pub(crate) fn calculate_height_for_width(width: u32, aspect_num: u32, aspect_den: u32) -> u32 {
    if aspect_num == 0 || aspect_den == 0 {
        return 1;
    }

    let computed = (width as f64 * aspect_den as f64 / aspect_num as f64).round() as u32;
    computed.max(1)
}

pub(crate) fn calculate_width_for_height(height: u32, aspect_num: u32, aspect_den: u32) -> u32 {
    if aspect_num == 0 || aspect_den == 0 {
        return 1;
    }

    let computed = (height as f64 * aspect_num as f64 / aspect_den as f64).round() as u32;
    computed.max(1)
}

fn reduce_ratio(width: u32, height: u32) -> (u32, u32) {
    let divisor = gcd(width.max(1), height.max(1));
    (width.max(1) / divisor, height.max(1) / divisor)
}

fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let tmp = left % right;
        left = right;
        right = tmp;
    }

    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resolution_works_for_valid_value() {
        assert_eq!(parse_resolution(Some("1920x1080")), Some((1920, 1080)));
    }

    #[test]
    fn parse_resolution_rejects_invalid_values() {
        assert_eq!(parse_resolution(None), None);
        assert_eq!(parse_resolution(Some("")), None);
        assert_eq!(parse_resolution(Some("1920")), None);
        assert_eq!(parse_resolution(Some("0x1080")), None);
        assert_eq!(parse_resolution(Some("badxvalue")), None);
    }

    #[test]
    fn locked_aspect_recomputes_height_from_width() {
        let height = calculate_height_for_width(1920, 16, 9);
        assert_eq!(height, 1080);
    }

    #[test]
    fn locked_aspect_recomputes_width_from_height() {
        let width = calculate_width_for_height(1080, 16, 9);
        assert_eq!(width, 1920);
    }

    #[test]
    fn gop_default_uses_fps_times_two() {
        let metadata = VideoMetadata {
            framerate: Some(29.97),
            ..VideoMetadata::default()
        };

        let form = EncodeOptionsForm::from_metadata(&metadata);
        assert_eq!(form.gop, 60);

        let metadata_25 = VideoMetadata {
            framerate: Some(25.0),
            ..VideoMetadata::default()
        };
        let form_25 = EncodeOptionsForm::from_metadata(&metadata_25);
        assert_eq!(form_25.gop, 50);

        let metadata_none = VideoMetadata::default();
        let form_none = EncodeOptionsForm::from_metadata(&metadata_none);
        assert_eq!(form_none.gop, 60);
    }

    #[test]
    fn variant_defaults_are_resolution_agnostic() {
        let form = EncodeOptionsForm::from_metadata(&VideoMetadata::default());

        assert_eq!(form.variants.len(), 3);
        assert_eq!(form.variants[0].name, "low");
        assert_eq!(form.variants[1].name, "med");
        assert_eq!(form.variants[2].name, "high");
    }

    #[test]
    fn categorical_defaults_match_script_values() {
        let form = EncodeOptionsForm::from_metadata(&VideoMetadata::default());

        assert_eq!(form.video_codec_lib, VideoCodecLib::X264);
        assert_eq!(form.profile, VideoProfile::High);
        assert_eq!(form.preset, X264Preset::Veryfast);
        assert_eq!(form.sc_threshold, 0);
        assert_eq!(form.audio_codec, AudioCodec::Aac);
        assert_eq!(form.hls_playlist_type, HlsPlaylistType::Vod);
    }

    #[test]
    fn codec_selection_updates_profile_preset_and_sc_threshold() {
        let mut form = EncodeOptionsForm::from_metadata(&VideoMetadata::default());

        form.apply_codec_defaults(VideoCodecLib::X265);
        assert_eq!(form.profile, VideoProfile::Main);
        assert_eq!(form.preset, X264Preset::Medium);
        assert_eq!(form.sc_threshold, 0);

        form.apply_codec_defaults(VideoCodecLib::Vp9);
        assert_eq!(form.profile, VideoProfile::Profile0);
        assert_eq!(form.preset, X264Preset::Good);
        assert_eq!(form.sc_threshold, 40);

        form.apply_codec_defaults(VideoCodecLib::Av1);
        assert_eq!(form.profile, VideoProfile::Main);
        assert_eq!(form.preset, X264Preset::Medium);
        assert_eq!(form.sc_threshold, 40);
    }

    #[test]
    fn output_defaults_are_initialized() {
        let form = EncodeOptionsForm::from_metadata(&VideoMetadata::default());

        assert_eq!(form.output_base_folder, "out");
        assert_eq!(form.output_subfolder_name, "myvideo123");
        assert_eq!(form.output_master_playlist_file, "master.m3u8");
    }
}
