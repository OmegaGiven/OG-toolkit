//! Reads link state straight out of /sys/class/net rather than shelling to
//! nmcli/iwctl — those manage connections, this module only needs to know
//! "up, and is it wifi or ethernet", which sysfs already has for free.
//! SSID lookup (`iw dev <if> link`) is the one place this still shells out,
//! since sysfs has no ESSID attribute.

use iced::widget::{container, mouse_area, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

pub fn is_wifi_powered() -> bool {
    let out = std::process::Command::new("rfkill")
        .args(["list", "wifi"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_default();
    !out.contains("Soft blocked: yes") && !out.is_empty()
}

#[derive(Debug, Clone, PartialEq)]
enum LinkState {
    Wifi(String),
    Ethernet,
    Disconnected,
}

fn detect_link() -> LinkState {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return LinkState::Disconnected;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        // Bridges/veth pairs/tailscale/docker0 are all "up" just as often as
        // the real NIC and sort before it as often as not — the `device`
        // symlink only exists for interfaces backed by real hardware, which
        // is the only thing this module should ever report on.
        if !entry.path().join("device").exists() {
            continue;
        }
        let operstate_path = entry.path().join("operstate");
        let Ok(state) = std::fs::read_to_string(&operstate_path) else { continue };
        if state.trim() != "up" {
            continue;
        }
        let is_wifi = entry.path().join("wireless").exists();
        if is_wifi {
            let essid = std::process::Command::new("iw")
                .args(["dev", &name, "link"])
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .and_then(|s| {
                    s.lines()
                        .find_map(|l| l.trim().strip_prefix("SSID: ").map(str::to_string))
                })
                .unwrap_or_else(|| "connected".to_string());
            return LinkState::Wifi(essid);
        }
        return LinkState::Ethernet;
    }
    LinkState::Disconnected
}

pub struct Network {
    state: LinkState,
    hovered: bool,
}

impl Network {
    pub fn new() -> Self {
        Self { state: detect_link(), hovered: false }
    }
}

impl Module for Network {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        let hovered = self.hovered;
        let icon = match &self.state {
            LinkState::Wifi(_) => "\u{f05a9}",
            LinkState::Ethernet => "\u{f0200}",
            LinkState::Disconnected => "\u{f05aa}",
        };
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
        .on_press(Message::Launch("og-settings --tab network".to_string()))
        .on_right_press(Message::WifiTogglePower)
        .on_enter(Message::NetworkHover(true))
        .on_exit(Message::NetworkHover(false))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(10)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick | Message::WifiTogglePower = message {
            self.state = detect_link();
        }
        if let Message::NetworkHover(v) = message {
            self.hovered = *v;
        }
    }
}
