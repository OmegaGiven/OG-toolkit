use iced::widget::{button, column, container, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

fn read_volume_pct() -> Option<u32> {
    let out = std::process::Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    // e.g. "Volume: front-left: 45000 /  69% / -8.31 dB, ..."
    let pct_str = text.split('/').nth(1)?.trim().trim_end_matches('%');
    pct_str.trim().parse().ok()
}

fn read_muted() -> bool {
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
}

impl Pulseaudio {
    pub fn new() -> Self {
        Self { volume_pct: read_volume_pct().unwrap_or(0), muted: read_muted() }
    }
}

impl Module for Pulseaudio {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        // Icon glyph and the "MUTE"/percentage text can't share one Text
        // widget — Symbols Nerd Font has no ASCII/digit glyphs at all, so a
        // joined string tofu's everything but the icon. Two widgets, two
        // fonts, stacked.
        let (icon, sub) = if self.muted { ("\u{f0581}", "MUTE".to_string()) } else { ("\u{f057e}", format!("{}%", self.volume_pct)) };
        button(
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
        .padding(0)
        .style(move |_, status| button::Style {
            background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                colors.header_btn_bg
            } else {
                Color::TRANSPARENT
            })),
            border: Border { radius: colors.radius.into(), ..Default::default() },
            text_color: fg,
            ..Default::default()
        })
        .on_press(Message::Launch("pactl set-sink-mute @DEFAULT_SINK@ toggle".to_string()))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(3)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick = message {
            self.volume_pct = read_volume_pct().unwrap_or(self.volume_pct);
            self.muted = read_muted();
        }
    }
}
