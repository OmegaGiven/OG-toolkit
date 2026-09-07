use iced::{
    widget::{button, column, container, progress_bar, row, scrollable, text},
    Background, Border, Color, Element, Length, Padding,
};
use std::path::PathBuf;
use crate::app::{Message, ACCENT, BG, SEC_BG, TEXT, MUTED};
use crate::filesystem::{home_dir, format_size, DriveInfo};

const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

pub fn bookmarks() -> Vec<(&'static str, &'static str, PathBuf)> {
    let home = home_dir();
    vec![
        ("\u{f015}", "Home",      home.clone()),
        ("\u{f108}", "Desktop",   home.join("Desktop")),
        ("\u{f019}", "Downloads", home.join("Downloads")),
        ("\u{f0f6}", "Documents", home.join("Documents")),
        ("\u{f1c5}", "Pictures",  home.join("Pictures")),
        ("\u{f001}", "Music",     home.join("Music")),
        ("\u{f1c8}", "Videos",    home.join("Videos")),
        ("\u{f1f8}", "Trash",     home.join(".local/share/Trash/files")),
    ]
}

fn section_label<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(11).style(move |_| text::Style { color: Some(MUTED()) }))
        .padding(Padding { top: 10.0, bottom: 4.0, left: 8.0, right: 8.0 })
        .into()
}

fn nav_button<'a>(icon: &'a str, label: String, path: PathBuf, is_active: bool) -> Element<'a, Message> {
    let content = row![
        container(text(icon).font(ICON_FONT).size(14).style(move |_| text::Style {
            color: Some(if is_active { BG() } else { TEXT() }),
        })).width(18),
        text(label).size(13),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    button(content)
        .on_press(Message::Navigate(path))
        .style(move |_, status| {
            let bg = if is_active {
                ACCENT()
            } else {
                match status {
                    button::Status::Hovered => Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 },
                    _ => SEC_BG(),
                }
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: if is_active { BG() } else { TEXT() },
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .into()
}

fn drive_entry<'a>(icon: &'a str, drive: &'a DriveInfo, current_path: &PathBuf) -> Element<'a, Message> {
    let is_active = current_path == &drive.mount_point;
    let fraction = drive.used_fraction();

    let header = row![
        container(text(icon).font(ICON_FONT).size(14).style(move |_| text::Style {
            color: Some(if is_active { BG() } else { TEXT() }),
        })).width(18),
        text(&drive.label).size(13),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let usage_text = text(format!("{} / {}", format_size(drive.used), format_size(drive.total)))
        .size(10)
        .style(move |_| text::Style { color: Some(if is_active { BG() } else { MUTED() }) });

    let bar = progress_bar(0.0..=1.0, fraction)
        .height(4)
        .style(move |_| progress_bar::Style {
            background: Background::Color(if is_active {
                Color { r: 0.0, g: 0.0, b: 0.0, a: 0.15 }
            } else {
                Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 }
            }),
            bar: Background::Color(if is_active { BG() } else { ACCENT() }),
            border: Border { radius: 2.0.into(), ..Default::default() },
        });

    let content = column![header, bar, usage_text].spacing(4).width(Length::Fill);

    button(content)
        .on_press(Message::Navigate(drive.mount_point.clone()))
        .padding([6, 8])
        .style(move |_, status| {
            let bg = if is_active {
                ACCENT()
            } else {
                match status {
                    button::Status::Hovered => Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 },
                    _ => SEC_BG(),
                }
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: if is_active { BG() } else { TEXT() },
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .into()
}

pub fn view<'a>(
    current_path: &PathBuf,
    devices: &'a [DriveInfo],
    network: &'a [DriveInfo],
    recent: &'a [PathBuf],
    user_bookmarks: &'a [PathBuf],
    width: f32,
) -> Element<'a, Message> {
    let mut col = column![].spacing(2).padding([8, 4]);

    col = col.push(section_label("Places"));
    for (icon, label, path) in bookmarks() {
        let is_active = current_path == &path;
        col = col.push(nav_button(icon, label.to_string(), path, is_active));
    }

    col = col.push(section_label("Bookmarks"));
    if user_bookmarks.is_empty() {
        col = col.push(
            container(text("Right-click a folder to bookmark it").size(11).style(move |_| text::Style { color: Some(MUTED()) }))
                .padding([2, 8]),
        );
    }
    for path in user_bookmarks {
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let is_active = current_path == path;
        col = col.push(nav_button("\u{f02e}", label, path.clone(), is_active));
    }

    col = col.push(section_label("Devices"));
    if devices.is_empty() {
        col = col.push(
            container(text("No devices").size(12).style(move |_| text::Style { color: Some(MUTED()) }))
                .padding([2, 8]),
        );
    }
    for drive in devices {
        col = col.push(drive_entry("\u{f0a0}", drive, current_path));
    }

    col = col.push(section_label("Network"));
    if network.is_empty() {
        col = col.push(
            container(text("No network drives").size(12).style(move |_| text::Style { color: Some(MUTED()) }))
                .padding([2, 8]),
        );
    }
    for drive in network {
        col = col.push(drive_entry("\u{f0ac}", drive, current_path));
    }

    col = col.push(section_label("Recent"));
    if recent.is_empty() {
        col = col.push(
            container(text("No recent places").size(12).style(move |_| text::Style { color: Some(MUTED()) }))
                .padding([2, 8]),
        );
    }
    for path in recent {
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let is_active = current_path == path;
        col = col.push(nav_button("\u{f017}", label, path.clone(), is_active));
    }

    container(scrollable(col))
        .width(width)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SEC_BG())),
            ..Default::default()
        })
        .into()
}
