use crate::ff_helpers::{PreviewVideo, VideoMetadata};
use crate::message::*;
use crate::views;

use iced::widget::markdown;
use iced::{Element, Task, Theme};
use std::fmt;

#[derive(Default)]
pub(crate) enum AppState {
    #[default]
    Initial,
    VideoOverview,
    EncodeOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone)]
pub(crate) struct VariantForm {
    pub name: String,
    pub video_bitrate_k: u32,
    pub maxrate_k: u32,
    pub bufsize_k: u32,
    pub audio_bitrate_k: u32,
}

impl VariantForm {
    fn defaults() -> [Self; 3] {
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

#[derive(Default)]
pub(crate) struct HLSenpai {
    pub video: Option<PreviewVideo>,
    pub state: AppState,
    pub encode_options: Option<EncodeOptionsForm>,
    pub ffmpeg_script_popup: Option<markdown::Content>,
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
            AppState::EncodeOptions => views::encode_options(self),
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
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
