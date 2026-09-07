use iced::widget::{button, container, stack, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

/// Counts visible notifications the same way og-notif-center does: mako's
/// own history minus whatever's been locally hidden. The hidden-ids file is
/// the shared contract between the two (see og-notif-center's
/// `notif.rs`) — read here rather than duplicating og-notif-center's
/// dismiss/restore logic, since this module only ever needs a count.
fn unread_count() -> u32 {
    let home = std::env::var("HOME").unwrap_or_default();
    let hidden_path = format!("{home}/.local/share/notification-hidden.json");
    let hidden: std::collections::HashSet<String> = std::fs::read_to_string(&hidden_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let Ok(out) = std::process::Command::new("makoctl").args(["history", "-j"]).output() else {
        return 0;
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return 0;
    };
    let Some(arr) = json.as_array() else { return 0 };

    arr.iter().filter(|entry| !hidden.contains(&entry.to_string())).count() as u32
}

pub struct Notifications {
    count: u32,
}

impl Notifications {
    pub fn new() -> Self {
        Self { count: unread_count() }
    }
}

impl Module for Notifications {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        let icon = container(
            text("\u{f0f3}").size(16).font(icon_font::nerd_font()).style(move |_| text::Style { color: Some(fg) }),
        )
        .width(size as u16)
        .height(size as u16)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        let content: Element<'_, Message> = if self.count > 0 {
            let badge_label = if self.count > 99 { "99+".to_string() } else { self.count.to_string() };
            let badge = container(text(badge_label).size(9).style(move |_| text::Style { color: Some(Color::WHITE) }))
                .padding([1, 4])
                .style(move |_| container::Style {
                    background: Some(Background::Color(colors.accent)),
                    border: Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                });
            stack![icon, container(badge).align_right(Length::Fill).align_top(Length::Fill)].into()
        } else {
            icon.into()
        };

        button(content)
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
            // The `-launch` wrapper (not the raw binary) — it grabs a
            // compositor-level Escape bind for as long as the popup is
            // open, coordinated with og-search-launch's identical
            // mechanism via og-popup-escape-guard, so Escape closes this
            // even when it isn't the focused window.
            .on_press(Message::Launch("og-notif-center-launch".to_string()))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(5)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: &Message) {
        if let Message::Tick = message {
            self.count = unread_count();
        }
    }
}
