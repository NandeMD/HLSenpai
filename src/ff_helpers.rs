use ffmpeg_next as ffmpeg;
use std::path::Path;

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
