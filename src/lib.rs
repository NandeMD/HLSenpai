mod app;
mod ff_helpers;
mod message;

pub fn run() -> iced::Result {
    iced::application(
        app::HLSenpai::new,
        app::HLSenpai::update,
        app::HLSenpai::view,
    )
    .run()
}
