//! StatusNotifierItem tray — the actual point of the step-8 spike
//! (src/bin/tray_spike.rs): apps like Steam/Discord that keep running with
//! just a tray icon after their window closes. This is the module that
//! actually renders those icons and lets you click them, not Taskbar
//! (which only shows windows that are still open).
//!
//! Right-click / DBusMenu context-menu rendering is NOT built yet — left-
//! click (Activate) only. Flagged here, not silently dropped: rendering an
//! arbitrary nested DBusMenu tree as its own popup is a real chunk of work
//! by itself, deferred rather than rushed into this pass.

use iced::widget::{button, container, image, text};
use iced::{Background, Border, Color, Element, Length, Subscription};

use system_tray::client::{ActivateRequest, Client};
use system_tray::item::IconPixmap;

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

#[derive(Debug, Clone)]
pub enum TrayIcon {
    Rgba { width: u32, height: u32, pixels: Vec<u8> },
    /// Loaded from an icon theme file (PNG bytes, passed straight to
    /// iced::widget::image which sniffs the format).
    File(Vec<u8>),
    /// Nothing usable was found — fall back to a generic glyph rather than
    /// showing nothing at all.
    Glyph,
}

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub address: String,
    pub icon: TrayIcon,
    pub title: String,
}

/// SNI icon pixmaps are ARGB32 in network byte order (A,R,G,B per pixel) —
/// iced's Handle::from_rgba wants R,G,B,A. Picks the largest available
/// size since a tray icon this small never needs more than that anyway.
fn pixmap_to_icon(pixmaps: &[IconPixmap]) -> Option<TrayIcon> {
    let biggest = pixmaps.iter().max_by_key(|p| p.width * p.height)?;
    if biggest.width <= 0 || biggest.height <= 0 {
        return None;
    }
    let mut rgba = Vec::with_capacity(biggest.pixels.len());
    for chunk in biggest.pixels.chunks_exact(4) {
        let [a, r, g, b] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    Some(TrayIcon::Rgba { width: biggest.width as u32, height: biggest.height as u32, pixels: rgba })
}

/// Shallow (2-level) search for `{icon_name}.png` under a directory —
/// icon themes nest by size/category (e.g. `48x48/apps/name.png`) and
/// there's no single fixed path, so this just looks rather than guessing
/// one exact layout.
fn find_icon_file(root: &std::path::Path, icon_name: &str, depth: u32) -> Option<std::path::PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_stem().and_then(|s| s.to_str()) == Some(icon_name) && path.extension().and_then(|e| e.to_str()) == Some("png") {
            return Some(path);
        }
    }
    for dir in subdirs {
        if let Some(found) = find_icon_file(&dir, icon_name, depth - 1) {
            return Some(found);
        }
    }
    None
}

fn resolve_icon(icon_name: Option<&str>, icon_theme_path: Option<&str>, pixmaps: Option<&[IconPixmap]>) -> TrayIcon {
    if let Some(pixmaps) = pixmaps {
        if let Some(icon) = pixmap_to_icon(pixmaps) {
            return icon;
        }
    }
    if let Some(name) = icon_name {
        let search_roots: Vec<std::path::PathBuf> = icon_theme_path
            .map(std::path::PathBuf::from)
            .into_iter()
            .chain(["/usr/share/icons/hicolor", "/usr/share/pixmaps"].map(std::path::PathBuf::from))
            .collect();
        for root in search_roots {
            if let Some(path) = find_icon_file(&root, name, 3) {
                if let Ok(bytes) = std::fs::read(&path) {
                    return TrayIcon::File(bytes);
                }
            }
        }
    }
    TrayIcon::Glyph
}

/// Pure snapshot -> Vec<TrayItem>, given an already-connected client's own
/// item map — no dbus I/O here. The map key is the item's DBus bus address
/// (e.g. ":1.370"), needed for Activate; `item.id` is a separate, app-
/// chosen SNI id (e.g. "steam") that isn't usable for that.
fn snapshot_items(items: &system_tray::data::BaseMap) -> Vec<TrayItem> {
    items
        .iter()
        .map(|(address, (item, _menu))| TrayItem {
            address: address.clone(),
            icon: resolve_icon(item.icon_name.as_deref(), item.icon_theme_path.as_deref(), item.icon_pixmap.as_deref()),
            title: item.title.clone().unwrap_or_default(),
        })
        .collect()
}

pub struct Tray {
    items: Vec<TrayItem>,
}

impl Tray {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl Module for Tray {
    fn cell_length(&self, _size: u32, _orientation: Orientation) -> Length {
        Length::Shrink
    }

    fn view(&self, colors: AppColors, size: u32, orientation: Orientation) -> Element<'_, Message> {
        let cells: Vec<Element<Message>> = self
            .items
            .iter()
            .map(|item| {
                let content: Element<Message> = match &item.icon {
                    TrayIcon::Rgba { width, height, pixels } => {
                        image(image::Handle::from_rgba(*width, *height, pixels.clone())).width(16).height(16).into()
                    }
                    TrayIcon::File(bytes) => image(image::Handle::from_bytes(bytes.clone())).width(16).height(16).into(),
                    TrayIcon::Glyph => {
                        let fg = colors.text;
                        text("\u{f2d0}").size(14).font(icon_font::nerd_font()).style(move |_| text::Style { color: Some(fg) }).into()
                    }
                };
                let btn = button(container(content).width(size as u16).height(size as u16).center_x(Length::Fill).center_y(Length::Fill))
                    .padding(0)
                    .style(move |_, status| iced::widget::button::Style {
                        background: Some(Background::Color(if matches!(status, iced::widget::button::Status::Hovered) {
                            colors.header_btn_bg
                        } else {
                            Color::TRANSPARENT
                        })),
                        border: Border { radius: colors.radius.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .on_press(Message::TrayActivate(item.address.clone()));

                if item.title.is_empty() {
                    btn.into()
                } else {
                    iced::widget::tooltip(
                        btn,
                        container(text(item.title.clone()).size(12).style(move |_| text::Style { color: Some(colors.text) }))
                            .padding(6)
                            .style(move |_| container::Style {
                                background: Some(Background::Color(colors.surface)),
                                border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                                ..Default::default()
                            }),
                        iced::widget::tooltip::Position::Bottom,
                    )
                    .into()
                }
            })
            .collect();

        match orientation {
            Orientation::Horizontal => iced::widget::row(cells).spacing(2).into(),
            Orientation::Vertical => iced::widget::column(cells).spacing(2).into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run_with_id("tray", tray_stream())
    }

    fn update(&mut self, message: &Message) {
        if let Message::TrayUpdated(items) = message {
            self.items = items.clone();
        }
    }
}

fn tray_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, |mut sender| async move {
        use iced::futures::SinkExt;

        let Ok(client) = Client::new().await else { return };
        let mut rx = client.subscribe();
        let items = client.items();

        // Real bug found while testing this: creating a *second* fresh
        // Client (with its own short sleep-then-read) on every fetch,
        // instead of reusing this already-connected one, meant every
        // refresh raced a brand-new connection's own replay from the
        // watcher — usually losing that race, so nothing ever rendered.
        // Reading straight from this client's own `items()` handle avoids
        // that: it only ever reflects events *this* connection actually
        // received.
        let snapshot = { snapshot_items(&items.lock().unwrap()) };
        let _ = sender.send(Message::TrayUpdated(snapshot)).await;

        while rx.recv().await.is_ok() {
            let snapshot = { snapshot_items(&items.lock().unwrap()) };
            let _ = sender.send(Message::TrayUpdated(snapshot)).await;
        }
    })
}

pub async fn activate(address: String) {
    if let Ok(client) = Client::new().await {
        let _ = client.activate(ActivateRequest::Default { address, x: 0, y: 0 }).await;
    }
}
