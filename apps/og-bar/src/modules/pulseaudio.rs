use iced::widget::{column, container, mouse_area, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

pub fn read_volume_pct() -> Option<u32> {
    let out = std::process::Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    // e.g. "Volume: front-left: 45000 /  69% / -8.31 dB, ..."
    let pct_str = text.split('/').nth(1)?.trim().trim_end_matches('%');
    pct_str.trim().parse().ok()
}

pub fn read_muted() -> bool {
    std::process::Command::new("pactl")
        .args(["get-sink-mute", "@DEFAULT_SINK@"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.contains("yes"))
        .unwrap_or(false)
}

pub struct Pulseaudio {
    volume_pct: u32,
    muted: bool,
    hovered: bool,
}

impl Pulseaudio {
    pub fn new() -> Self {
        Self { volume_pct: read_volume_pct().unwrap_or(0), muted: read_muted(), hovered: false }
    }
}

impl Module for Pulseaudio {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = if self.muted { Color::WHITE } else { colors.text };
        // Icon glyph and the percentage text can't share one Text widget —
        // Symbols Nerd Font has no ASCII/digit glyphs at all, so a joined
        // string tofu's everything but the icon. Two widgets, two fonts,
        // stacked. Percentage stays visible even muted — mute state is
        // the icon's job, hiding the number too just loses information.
        let icon = if self.muted { "\u{f0581}" } else { "\u{f057e}" };
        let sub = format!("{}%", self.volume_pct);
        mouse_area(
            container(
                container(
                    column![
                        text(icon).size(14).font(icon_font::nerd_font()).align_x(iced::alignment::Horizontal::Center).style(move |_| text::Style { color: Some(fg) }),
                        text(sub).size(11).align_x(iced::alignment::Horizontal::Center).style(move |_| text::Style { color: Some(fg) }),
                    ]
                    .align_x(iced::Alignment::Center),
                )
                .width(size as u16)
                .height(size as u16)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            )
            .style(move |_| container::Style {
                border: Border { radius: colors.radius.into(), ..Default::default() },
                // Muted red wins over hover — same danger-red og-settings'
                // Audio tab uses for its mute button, so a muted state
                // reads the same way from either app.
                background: Some(Background::Color(if self.muted {
                    Color { r: 0.6, g: 0.1, b: 0.1, a: 1.0 }
                } else if self.hovered {
                    colors.header_btn_bg
                } else {
                    Color::TRANSPARENT
                })),
                ..Default::default()
            }),
        )
        .on_press(Message::PulseaudioToggleMute)
        .on_right_press(Message::Launch("og-settings --tab audio".to_string()))
        .on_enter(Message::PulseaudioHover(true))
        .on_exit(Message::PulseaudioHover(false))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(3)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick | Message::PulseaudioToggleMute = message {
            self.volume_pct = read_volume_pct().unwrap_or(self.volume_pct);
            self.muted = read_muted();
        }
        if let Message::PulseaudioHover(v) = message {
            self.hovered = *v;
        }
    }
}
