use ffmpeg_next as ffmpeg;
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

pub struct PreviewVideo {
    pub _path: PathBuf,
    pub video: Video,
    pub position: f64,
    pub dragging: bool,
}
