use crate::app::{AudioCodec, EncodeRuntimeState, HLSenpai, HlsPlaylistType, VideoCodecLib};
use crate::message::Message;
use iced::widget::{
    button, checkbox, column, container, markdown, opaque, pick_list, progress_bar, row,
    scrollable, slider, stack, text, text_input,
};
use iced::{Alignment, Background, Element, Length};
use iced_video_player::VideoPlayer;
use std::time::Duration;

type El<'a> = Element<'a, Message>;

pub(crate) fn select_file(_app: &HLSenpai) -> El<'_> {
    let content = column![button("Select File").on_press(Message::SelectFilePressed),]
        .spacing(16)
        .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub(crate) fn video_overview(app: &HLSenpai) -> El<'_> {
    let content: El<'_> = match app.video.as_ref() {
        Some(video) => {
            let duration_secs = video.video.duration().as_secs_f64();
            let slider_value = video.position.clamp(0.0, duration_secs);

            let video_panel = column![
                text(format!(
                    "Video Preview ({})",
                    video
                        .metadata
                        .video_codec
                        .as_deref()
                        .unwrap_or("Unknown codec")
                ))
                .size(24),
                container(
                    VideoPlayer::new(&video.video)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Contain)
                        .on_end_of_stream(Message::EndOfStream)
                        .on_new_frame(Message::NewFrame),
                )
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill),
                container(
                    slider(0.0..=duration_secs, slider_value, Message::Seek)
                        .step(0.1)
                        .on_release(Message::SeekRelease),
                )
                .padding(iced::Padding::new(5.0).left(10.0).right(10.0)),
                row![
                    button(if video.video.paused() {
                        "Play"
                    } else {
                        "Pause"
                    })
                    .width(80.0)
                    .on_press(Message::TogglePause),
                    button(if video.video.looping() {
                        "Disable Loop"
                    } else {
                        "Enable Loop"
                    })
                    .width(120.0)
                    .on_press(Message::ToggleLoop),
                    text(format!(
                        "{} / {}",
                        format_time(slider_value),
                        format_time(duration_secs)
                    ))
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                ]
                .spacing(5)
                .align_y(iced::alignment::Vertical::Center)
                .padding(iced::Padding::new(10.0).top(0.0)),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill);

            let header = container(button("Encode Options").on_press(Message::OpenEncodeOptions))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right);

            let metadata_sections_row = if video.metadata_markdown_sections.is_empty() {
                row![text("No video metadata available.")]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
            } else {
                video.metadata_markdown_sections.iter().fold(
                    row![].spacing(12).height(Length::Fill),
                    |row, section| {
                        let section_card = container(
                            markdown::view(section.items(), iced::Theme::TokyoNightStorm)
                                .map(Message::MarkdownLinkClicked),
                        )
                        .width(Length::Fixed(360.0))
                        .height(Length::Fill)
                        .padding(iced::Padding::new(12.0))
                        .style(iced::widget::container::rounded_box);

                        row.push(section_card)
                    },
                )
            };

            let metadata_panel = container(scrollable(metadata_sections_row).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::default(),
                ),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(14.0));

            column![
                container(header)
                    .width(Length::Fill)
                    .padding(iced::Padding::new(12.0).right(14.0).top(8.0)),
                container(video_panel)
                    .width(Length::Fill)
                    .height(Length::FillPortion(3)),
                container(metadata_panel)
                    .width(Length::Fill)
                    .height(Length::FillPortion(2))
            ]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
        None => column![text("No video loaded. Select a file to continue.").size(24),]
            .spacing(16)
            .align_x(Alignment::Center)
            .into(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub(crate) fn encode_options(app: &HLSenpai) -> El<'_> {
    let content: El<'_> = match (app.video.as_ref(), app.encode_options.as_ref()) {
        (Some(video), Some(form)) => {
            let is_encode_running = app
                .encode_runtime
                .as_ref()
                .is_some_and(EncodeRuntimeState::is_running);
            let scale_width = form.scale_width.to_string();
            let scale_height = form.scale_height.to_string();
            let gop = form.gop.to_string();
            let sc_threshold = form.sc_threshold.to_string();
            let audio_channels = form.audio_channels.to_string();
            let hls_time = form.hls_time_seconds.to_string();

            let scale_section = container(
                column![
                    text("Scale").size(22),
                    text(format!(
                        "Source: {} ({})",
                        form.source_resolution_label(),
                        form.source_aspect_label()
                    )),
                    row![
                        text("Width").width(Length::Fixed(200.0)),
                        text_input("Width", &scale_width)
                            .on_input(Message::EncodeScaleWidthChanged)
                            .width(Length::Fixed(180.0)),
                        text("px")
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Height").width(Length::Fixed(200.0)),
                        text_input("Height", &scale_height)
                            .on_input(Message::EncodeScaleHeightChanged)
                            .width(Length::Fixed(180.0)),
                        text("px")
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    checkbox(form.scale_lock_aspect)
                        .label("Lock aspect ratio")
                        .on_toggle(Message::EncodeScaleLockToggled)
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Length::Fill);

            let codec_section = container(
                column![
                    text("Codec + GOP").size(22),
                    row![
                        text("Video codec lib").width(Length::Fixed(200.0)),
                        pick_list(
                            &VideoCodecLib::ALL[..],
                            Some(form.video_codec_lib),
                            Message::EncodeVideoCodecLibSelected
                        )
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Profile").width(Length::Fixed(200.0)),
                        pick_list(
                            form.profile_options(),
                            Some(form.profile),
                            Message::EncodeProfileSelected
                        )
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Preset").width(Length::Fixed(200.0)),
                        pick_list(
                            form.preset_options(),
                            Some(form.preset),
                            Message::EncodePresetSelected
                        )
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("sc_threshold").width(Length::Fixed(200.0)),
                        text_input("0", &sc_threshold)
                            .on_input(Message::EncodeScThresholdChanged)
                            .width(Length::Fixed(180.0))
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("GOP").width(Length::Fixed(200.0)),
                        text_input("60", &gop)
                            .on_input(Message::EncodeGopChanged)
                            .width(Length::Fixed(180.0))
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Audio codec").width(Length::Fixed(200.0)),
                        pick_list(
                            &AudioCodec::ALL[..],
                            Some(form.audio_codec),
                            Message::EncodeAudioCodecSelected
                        )
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Audio channels").width(Length::Fixed(200.0)),
                        text_input("2", &audio_channels)
                            .on_input(Message::EncodeAudioChannelsChanged)
                            .width(Length::Fixed(180.0))
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Length::Fill);

            let packaging_section = container(
                column![
                    text("HLS Packaging").size(22),
                    row![
                        text("HLS time").width(Length::Fixed(200.0)),
                        text_input("6", &hls_time)
                            .on_input(Message::EncodeHlsTimeChanged)
                            .width(Length::Fixed(180.0)),
                        text("seconds")
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Playlist type").width(Length::Fixed(200.0)),
                        pick_list(
                            &HlsPlaylistType::ALL[..],
                            Some(form.hls_playlist_type),
                            Message::EncodePlaylistTypeSelected
                        )
                        .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    checkbox(form.hls_flags_independent_segments)
                        .label("independent_segments")
                        .on_toggle(Message::EncodeIndependentSegmentsToggled),
                    row![
                        text("Master playlist").width(Length::Fixed(200.0)),
                        text_input("master.m3u8", &form.master_playlist_name)
                            .on_input(Message::EncodeMasterPlaylistNameChanged)
                            .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Segment pattern").width(Length::Fixed(200.0)),
                        text_input("v%v/seg_%06d.ts", &form.segment_filename_pattern)
                            .on_input(Message::EncodeSegmentPatternChanged)
                            .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Variant playlist pattern").width(Length::Fixed(200.0)),
                        text_input("v%v/prog.m3u8", &form.output_variant_playlist_pattern)
                            .on_input(Message::EncodeOutputPlaylistPatternChanged)
                            .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Length::Fill);

            let output_section = container(
                column![
                    text("Output Options").size(22),
                    row![
                        text("Base folder").width(Length::Fixed(200.0)),
                        text_input("out", &form.output_base_folder)
                            .on_input(Message::EncodeOutputBaseFolderChanged)
                            .width(Length::Fill),
                        button("Browse...").on_press(Message::EncodePickOutputBaseFolderPressed)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Subfolder name").width(Length::Fixed(200.0)),
                        text_input("myvideo123", &form.output_subfolder_name)
                            .on_input(Message::EncodeOutputSubfolderChanged)
                            .width(Length::Fill)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        text("Master playlist file").width(Length::Fixed(200.0)),
                        text_input("master.m3u8", &form.output_master_playlist_file)
                            .on_input(Message::EncodeOutputMasterPlaylistFileChanged)
                            .width(Length::Fill),
                        button("Browse...")
                            .on_press(Message::EncodePickOutputMasterPlaylistFilePressed)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(14)
            .width(Length::Fill);

            let variants_section = form.variants.iter().enumerate().fold(
                column![text("Variants").size(22)].spacing(10),
                |column, (index, variant)| {
                    let video_bitrate = variant.video_bitrate_k.to_string();
                    let maxrate = variant.maxrate_k.to_string();
                    let bufsize = variant.bufsize_k.to_string();
                    let audio_bitrate = variant.audio_bitrate_k.to_string();

                    let variant_card = container(
                        column![
                            text(format!("Variant {}", index + 1)).size(18),
                            row![
                                text("Name").width(Length::Fixed(200.0)),
                                text_input("name", &variant.name)
                                    .on_input(move |value| {
                                        Message::EncodeVariantNameChanged(index, value)
                                    })
                                    .width(Length::Fill)
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                            row![
                                text("Video bitrate").width(Length::Fixed(200.0)),
                                text_input("1200", &video_bitrate)
                                    .on_input(move |value| {
                                        Message::EncodeVariantVideoBitrateChanged(index, value)
                                    })
                                    .width(Length::Fixed(180.0)),
                                text("k")
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                            row![
                                text("Maxrate").width(Length::Fixed(200.0)),
                                text_input("1500", &maxrate)
                                    .on_input(move |value| {
                                        Message::EncodeVariantMaxrateChanged(index, value)
                                    })
                                    .width(Length::Fixed(180.0)),
                                text("k")
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                            row![
                                text("Bufsize").width(Length::Fixed(200.0)),
                                text_input("2400", &bufsize)
                                    .on_input(move |value| {
                                        Message::EncodeVariantBufsizeChanged(index, value)
                                    })
                                    .width(Length::Fixed(180.0)),
                                text("k")
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                            row![
                                text("Audio bitrate").width(Length::Fixed(200.0)),
                                text_input("96", &audio_bitrate)
                                    .on_input(move |value| {
                                        Message::EncodeVariantAudioBitrateChanged(index, value)
                                    })
                                    .width(Length::Fixed(180.0)),
                                text("k")
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(10),
                    )
                    .padding(12)
                    .width(Length::Fill);

                    column.push(variant_card)
                },
            );

            let fps_text = video
                .metadata
                .framerate
                .map(|fps| format!("{fps:.3}"))
                .unwrap_or_else(|| "Unknown".to_string());

            let back_button = if is_encode_running {
                button("Back").width(100.0)
            } else {
                button("Back")
                    .on_press(Message::BackToVideoOverview)
                    .width(100.0)
            };

            let encode_button = if is_encode_running {
                button("Encode").style(iced::widget::button::danger)
            } else {
                button("Encode")
                    .style(iced::widget::button::danger)
                    .on_press(Message::EncodePressed)
            };

            let mut header = row![
                back_button,
                column![
                    text("Encode Options").size(30),
                    text(format!(
                        "Source: {} ({}) | FPS: {}",
                        form.source_resolution_label(),
                        form.source_aspect_label(),
                        fps_text
                    ))
                ]
                .spacing(4)
                .width(Length::Fill),
                encode_button,
                button("Print ffmpeg Script").on_press(Message::PrintFfmpegScript)
            ]
            .spacing(14)
            .align_y(Alignment::Center);

            if app.encode_runtime.is_some() && !app.show_encode_log_modal {
                header =
                    header.push(button("Show Encode Log").on_press(Message::EncodeLogModalOpen));
            }

            let base_content = column![
                header,
                scrollable(
                    column![
                        scale_section,
                        codec_section,
                        packaging_section,
                        output_section,
                        container(variants_section).padding(14).width(Length::Fill),
                    ]
                    .spacing(16),
                )
                .height(Length::Fill)
                .width(Length::Fill),
            ]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill);

            let base_layer = container(base_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::new(16.0));

            let mut layered: El<'_> = base_layer.into();

            if let Some(script_popup) = app.ffmpeg_script_popup.as_ref() {
                let popup_content = column![
                    row![
                        text("Generated ffmpeg script").size(24).width(Length::Fill),
                        button("Close").on_press(Message::CloseFfmpegScriptPopup)
                    ]
                    .align_y(Alignment::Center),
                    scrollable(
                        markdown::view(script_popup.items(), iced::Theme::TokyoNightStorm)
                            .map(Message::MarkdownLinkClicked)
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                ]
                .spacing(14)
                .width(Length::Fill)
                .height(Length::Fill);

                let popup_layer = container(
                    container(popup_content)
                        .padding(16)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .max_width(920)
                        .max_height(620)
                        .style(iced::widget::container::rounded_box),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::new(24.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|theme: &iced::Theme| {
                    let mut overlay = theme.extended_palette().background.base.color;
                    overlay.a = 0.65;

                    iced::widget::container::Style {
                        background: Some(Background::Color(overlay)),
                        ..iced::widget::container::Style::default()
                    }
                });

                layered = stack![layered, opaque(popup_layer)]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }

            if app.show_encode_log_modal
                && let Some(runtime) = app.encode_runtime.as_ref()
            {
                let progress_value = runtime.progress_percent.unwrap_or(0.0).clamp(0.0, 100.0);
                let progress_label = runtime
                    .progress_percent
                    .map(|value| format!("{value:.1}%"))
                    .unwrap_or_else(|| "Calculating...".to_string());
                let out_time_label = runtime
                    .last_out_time_ms
                    .map(format_output_time)
                    .unwrap_or_else(|| "Unknown".to_string());
                let elapsed_label = format_duration(runtime.started_at.elapsed());
                let speed_label = runtime.speed.as_deref().unwrap_or("Unknown");
                let bitrate_label = runtime.bitrate.as_deref().unwrap_or("Unknown");
                let logs_text = if runtime.log_lines.is_empty() {
                    "Waiting for ffmpeg output...".to_string()
                } else {
                    runtime.log_lines.join("\n")
                };

                let cancel_button = if runtime.can_cancel() {
                    button("Cancel Encode")
                        .style(iced::widget::button::danger)
                        .on_press(Message::EncodeCancelPressed)
                } else {
                    button("Cancel Encode").style(iced::widget::button::danger)
                };

                let log_popup_content = column![
                    row![
                        text("Encode Output").size(24).width(Length::Fill),
                        button("Close").on_press(Message::EncodeLogModalClose)
                    ]
                    .align_y(Alignment::Center),
                    text(format!("Status: {}", runtime.status_label())).size(18),
                    row![
                        text(format!("Progress: {progress_label}")),
                        text(format!("Elapsed: {elapsed_label}")),
                        text(format!("Encoded time: {out_time_label}")),
                        text(format!("Speed: {speed_label}")),
                        text(format!("Bitrate: {bitrate_label}"))
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                    progress_bar(0.0..=100.0, progress_value),
                    container(
                        scrollable(text(logs_text).size(14))
                            .id("encode-log-scroll")
                            .width(Length::Fill)
                            .height(Length::Fill)
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
                    .style(iced::widget::container::rounded_box),
                    container(cancel_button)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                ]
                .spacing(14)
                .width(Length::Fill)
                .height(Length::Fill);

                let log_popup_layer = container(
                    container(log_popup_content)
                        .padding(16)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .max_width(980)
                        .max_height(700)
                        .style(iced::widget::container::rounded_box),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::new(24.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|theme: &iced::Theme| {
                    let mut overlay = theme.extended_palette().background.base.color;
                    overlay.a = 0.65;

                    iced::widget::container::Style {
                        background: Some(Background::Color(overlay)),
                        ..iced::widget::container::Style::default()
                    }
                });

                layered = stack![layered, opaque(log_popup_layer)]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }

            layered
        }
        _ => container(
            column![
                text("Encode options are not available.").size(24),
                button("Back").on_press(Message::BackToVideoOverview)
            ]
            .spacing(16)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn format_time(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}s", total_seconds / 60, total_seconds % 60)
}

fn format_output_time(out_time_ms: u64) -> String {
    let total_seconds = out_time_ms / 1_000_000;
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
