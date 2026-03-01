mod app;
mod ff_helpers;
mod message;
mod views;

pub fn run() -> iced::Result {
    iced::application(
        app::HLSenpai::new,
        app::HLSenpai::update,
        app::HLSenpai::view,
    )
    .theme(app::HLSenpai::theme)
    .run()
}
