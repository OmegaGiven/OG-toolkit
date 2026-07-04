mod app;
mod color_wheel;
mod config;
mod pty;
mod sway;
mod tabs;

use app::App;

fn main() -> iced::Result {
    iced::application("Settings Manager", App::update, App::view)
        .subscription(App::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 720.0),
            decorations: false,
            position: iced::window::Position::Centered,
            ..Default::default()
        })
        .run_with(App::new)
}
