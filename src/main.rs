mod app;
mod audio;
mod audio_meter;
mod color_wheel;
mod devices;
mod config;
mod cursor_theme;
mod galias;
mod printing;
mod pty;
mod sway;
mod tabs;
mod terminal_theme;
mod vpn;

use app::App;

fn main() -> iced::Result {
    iced::application("OG Settings", App::update, App::view)
        .subscription(App::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 720.0),
            decorations: false,
            position: iced::window::Position::Centered,
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "og-settings".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(App::new)
}
