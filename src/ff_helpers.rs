use ffmpeg_next as ffmpeg;
use iced::widget::markdown;
use iced_video_player::Video;
use std::path::{Path, PathBuf};

pub(crate) fn validate_video_file(path: &Path) -> Result<(), String> {
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

#[derive(Debug, Clone, Default)]
pub struct VideoMetadata {
    pub container_format: Option<String>,
    pub duration_seconds: Option<f64>,
    pub overall_bitrate: Option<u64>,
    pub file_metadata: Vec<(String, String)>,
    pub framerate: Option<f64>,
    pub framerate_ratio: Option<String>,
    pub video_resolution: Option<String>,
    pub video_ratio: Option<String>,
    pub video_bitrate: Option<u64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_bitrate: Option<u64>,
    pub video_pixel_format: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u16>,
    pub audio_channel_layout: Option<String>,
}

pub(crate) fn extract_video_metadata(path: &Path) -> Result<VideoMetadata, String> {
    ffmpeg::init().map_err(|err| format!("Failed to initialize ffmpeg: {err}"))?;

    let input = ffmpeg::format::input(path)
        .map_err(|err| format!("ffmpeg could not open the selected file: {err}"))?;

    let video_stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or_else(|| "No video stream was found in the selected file.".to_string())?;

    let video_codec_context =
        ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|err| format!("Could not read video stream parameters: {err}"))?;

    let video_decoder = video_codec_context
        .decoder()
        .video()
        .map_err(|err| format!("No supported video decoder found for this stream: {err}"))?;

    let chosen_framerate = {
        let avg = video_stream.avg_frame_rate();
        if is_valid_rational(avg) {
            Some(avg)
        } else {
            let raw = video_stream.rate();
            if is_valid_rational(raw) {
                Some(raw)
            } else {
                video_decoder
                    .frame_rate()
                    .filter(|value| is_valid_rational(*value))
            }
        }
    };

    let width = video_decoder.width();
    let height = video_decoder.height();
    let resolution = if width > 0 && height > 0 {
        Some(format!("{width}x{height}"))
    } else {
        None
    };

    let sample_aspect_ratio = video_decoder.aspect_ratio();
    let ratio = display_aspect_ratio(width, height, sample_aspect_ratio);

    let audio_stream = input.streams().best(ffmpeg::media::Type::Audio);

    let (audio_codec, audio_bitrate, audio_sample_rate, audio_channels, audio_channel_layout) =
        if let Some(stream) = audio_stream {
            let codec_name = Some(stream.parameters().id().name().to_string());

            let audio_values =
                ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                    .ok()
                    .and_then(|context| context.decoder().audio().ok());

            if let Some(decoder) = audio_values {
                (
                    codec_name,
                    normalize_bitrate(decoder.bit_rate() as i64),
                    Some(decoder.rate()),
                    Some(decoder.channels()),
                    Some(format!("{:?}", decoder.channel_layout())),
                )
            } else {
                (codec_name, None, None, None, None)
            }
        } else {
            (None, None, None, None, None)
        };

    Ok(VideoMetadata {
        container_format: Some(input.format().name().to_string()),
        duration_seconds: normalize_duration(input.duration()),
        overall_bitrate: normalize_bitrate(input.bit_rate()),
        file_metadata: input
            .metadata()
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        framerate: chosen_framerate.and_then(rational_to_f64),
        framerate_ratio: chosen_framerate.and_then(rational_to_string),
        video_resolution: resolution,
        video_ratio: ratio,
        video_bitrate: normalize_bitrate(video_decoder.bit_rate() as i64),
        video_codec: Some(video_stream.parameters().id().name().to_string()),
        audio_codec,
        audio_bitrate,
        video_pixel_format: Some(format!("{:?}", video_decoder.format())),
        audio_sample_rate,
        audio_channels,
        audio_channel_layout,
    })
}

fn normalize_duration(duration: i64) -> Option<f64> {
    if duration > 0 {
        Some(duration as f64 / ffmpeg::ffi::AV_TIME_BASE as f64)
    } else {
        None
    }
}

fn normalize_bitrate(bitrate: i64) -> Option<u64> {
    if bitrate > 0 {
        Some(bitrate as u64)
    } else {
        None
    }
}

fn is_valid_rational(value: ffmpeg::Rational) -> bool {
    value.numerator() > 0 && value.denominator() > 0
}

fn rational_to_f64(value: ffmpeg::Rational) -> Option<f64> {
    if is_valid_rational(value) {
        Some(value.numerator() as f64 / value.denominator() as f64)
    } else {
        None
    }
}

fn rational_to_string(value: ffmpeg::Rational) -> Option<String> {
    if is_valid_rational(value) {
        Some(format!("{}/{}", value.numerator(), value.denominator()))
    } else {
        None
    }
}

fn display_aspect_ratio(
    width: u32,
    height: u32,
    sample_aspect_ratio: ffmpeg::Rational,
) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }

    let (sar_num, sar_den) = if is_valid_rational(sample_aspect_ratio) {
        (
            sample_aspect_ratio.numerator() as u64,
            sample_aspect_ratio.denominator() as u64,
        )
    } else {
        (1, 1)
    };

    let mut numerator = width as u64 * sar_num;
    let mut denominator = height as u64 * sar_den;
    let gcd_value = gcd(numerator, denominator);
    numerator /= gcd_value;
    denominator /= gcd_value;

    Some(format!("{numerator}:{denominator}"))
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let tmp = left % right;
        left = right;
        right = tmp;
    }
    left.max(1)
}

pub(crate) fn video_metadata_markdown(metadata: &VideoMetadata) -> String {
    let mut markdown_text = format!(
        "# Video Information\n\
         \n\
         ## Video\n\
         - **Codec:** {}\n\
         - **Resolution:** {}\n\
         - **Aspect Ratio:** {}\n\
         - **Frame Rate:** {}\n\
         - **Bitrate:** {}\n\
         - **Pixel Format:** {}\n\
         \n\
         ## Audio\n\
         - **Codec:** {}\n\
         - **Bitrate:** {}\n\
         - **Sample Rate:** {}\n\
         - **Channels:** {}\n\
         - **Channel Layout:** {}\n\
         \n\
         ## Container\n\
         - **Format:** {}\n\
         - **Duration:** {}\n\
         - **Overall Bitrate:** {}\n\
         - **Tags:** {}\n",
        metadata.video_codec.as_deref().unwrap_or("Unknown"),
        metadata.video_resolution.as_deref().unwrap_or("Unknown"),
        metadata.video_ratio.as_deref().unwrap_or("Unknown"),
        format_framerate(metadata.framerate, metadata.framerate_ratio.as_deref()),
        format_bitrate(metadata.video_bitrate),
        metadata.video_pixel_format.as_deref().unwrap_or("Unknown"),
        metadata.audio_codec.as_deref().unwrap_or("None"),
        format_bitrate(metadata.audio_bitrate),
        format_hz(metadata.audio_sample_rate),
        metadata
            .audio_channels
            .map(|channels| channels.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        metadata
            .audio_channel_layout
            .as_deref()
            .unwrap_or("Unknown"),
        metadata.container_format.as_deref().unwrap_or("Unknown"),
        format_seconds(metadata.duration_seconds),
        format_bitrate(metadata.overall_bitrate),
        metadata.file_metadata.len(),
    );

    if !metadata.file_metadata.is_empty() {
        markdown_text.push_str("\n## Metadata Tags\n");
        for (key, value) in &metadata.file_metadata {
            markdown_text.push_str(&format!("- **{key}:** {value}\n"));
        }
    }

    markdown_text
}

fn format_seconds(seconds: Option<f64>) -> String {
    seconds
        .map(|value| format!("{value:.2}s"))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_bitrate(bitrate: Option<u64>) -> String {
    bitrate
        .map(|value| format!("{:.2} kbps", value as f64 / 1_000.0))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_hz(value: Option<u32>) -> String {
    value
        .map(|hz| format!("{hz} Hz"))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_framerate(framerate: Option<f64>, ratio: Option<&str>) -> String {
    match (framerate, ratio) {
        (Some(value), Some(ratio_value)) => format!("{value:.3} fps ({ratio_value})"),
        (Some(value), None) => format!("{value:.3} fps"),
        (None, Some(ratio_value)) => ratio_value.to_string(),
        (None, None) => "Unknown".to_string(),
    }
}

pub struct PreviewVideo {
    pub _path: PathBuf,
    pub video: Video,
    pub metadata: VideoMetadata,
    pub metadata_markdown: markdown::Content,
    pub position: f64,
    pub dragging: bool,
}
