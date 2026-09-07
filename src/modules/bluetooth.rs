use iced::widget::{button, column, container, mouse_area, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

/// Right-click menu actions — same set waybar's old `bluetooth-menu.xml`
/// offered. "Devices…"/"Adapter settings…"/"Send files…" shell out to
/// blueman rather than og-settings' own (fuller) bluetooth UI, since
/// that's literally what the old menu ran and this is a like-for-like
/// replacement, not a redesign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothAction {
    PowerOn,
    PowerOff,
    Devices,
    Adapters,
    SendFiles,
}

impl BluetoothAction {
    pub fn label(&self) -> &'static str {
        match self {
            BluetoothAction::PowerOn => "Turn adapter on",
            BluetoothAction::PowerOff => "Turn adapter off",
            BluetoothAction::Devices => "Devices…",
            BluetoothAction::Adapters => "Adapter settings…",
            BluetoothAction::SendFiles => "Send files…",
        }
    }

    pub fn run(&self) {
        match self {
            BluetoothAction::PowerOn => {
                let _ = std::process::Command::new("bluetoothctl").args(["power", "on"]).output();
            }
            BluetoothAction::PowerOff => {
                let _ = std::process::Command::new("bluetoothctl").args(["power", "off"]).output();
            }
            BluetoothAction::Devices => {
                let _ = std::process::Command::new("blueman-manager").spawn();
            }
            BluetoothAction::Adapters => {
                let _ = std::process::Command::new("blueman-adapters").spawn();
            }
            BluetoothAction::SendFiles => {
                let _ = std::process::Command::new("blueman-sendto").spawn();
            }
        }
    }
}

pub fn is_powered() -> bool {
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
    hovered: bool,
}

impl Bluetooth {
    pub fn new() -> Self {
        Self { powered: is_powered(), hovered: false }
    }
}

impl Module for Bluetooth {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        let hovered = self.hovered;
        let icon = if self.powered { "\u{f00af}" } else { "\u{f00b2}" };
        mouse_area(
            container(
                container(text(icon).size(16).font(icon_font::nerd_font()).style(move |_| text::Style { color: Some(fg) }))
                    .width(size as u16)
                    .height(size as u16)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .style(move |_| container::Style {
                background: Some(Background::Color(if hovered { colors.header_btn_bg } else { Color::TRANSPARENT })),
                border: Border { radius: colors.radius.into(), ..Default::default() },
                ..Default::default()
            }),
        )
        .on_press(Message::BluetoothTogglePower)
        .on_right_press(Message::OpenBluetoothMenu)
        .on_enter(Message::BluetoothHover(true))
        .on_exit(Message::BluetoothHover(false))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(10)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick | Message::BluetoothTogglePower = message {
            self.powered = is_powered();
        }
        if let Message::BluetoothHover(v) = message {
            self.hovered = *v;
        }
    }
}

/// Right-click popup — same button-list styling as `power::popup_view`.
pub fn popup_view(colors: AppColors) -> Element<'static, Message> {
    let actions = [
        BluetoothAction::PowerOn,
        BluetoothAction::PowerOff,
        BluetoothAction::Devices,
        BluetoothAction::Adapters,
        BluetoothAction::SendFiles,
    ];

    let buttons = actions.into_iter().map(|action| {
        let fg = colors.text;
        button(text(action.label()).size(14).style(move |_| text::Style { color: Some(fg) }))
            .width(Length::Fill)
            .padding(8)
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
            .on_press(Message::BluetoothAction(action))
            .into()
    });

    container(column(buttons).spacing(2).padding(6))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(colors.bg_fill),
            border: Border { color: colors.accent, width: 1.0, radius: colors.radius.into() },
            ..Default::default()
        })
        .into()
}
