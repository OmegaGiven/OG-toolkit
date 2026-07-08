use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

fn is_powered() -> bool {
    std::process::Command::new("bluetoothctl")
        .arg("show")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.lines().any(|l| l.trim() == "Powered: yes"))
        .unwrap_or(false)
}

pub struct Bluetooth {
    powered: bool,
}

impl Bluetooth {
    pub fn new() -> Self {
        Self { powered: is_powered() }
    }
}

impl Module for Bluetooth {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        let icon = if self.powered { "󰂯" } else { "󰂲" };
        button(
            container(text(icon).size(16).style(move |_| text::Style { color: Some(fg) }))
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
        .on_press(Message::Launch("~/.local/bin/og-settings --tab network".to_string()))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(10)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick = message {
            self.powered = is_powered();
        }
    }
}
