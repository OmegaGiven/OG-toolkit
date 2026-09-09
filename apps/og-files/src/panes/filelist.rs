use iced::widget::{button, column, container, image, mouse_area, row, scrollable, text};
use iced::{Background, Border, Color, Element, Length};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::app::Message;
use crate::filesystem::{format_size, FileEntry};
use crate::tab::{SortBy, ViewMode};
use crate::theme;

const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

/// Real decoded thumbnail (via the async cache in `thumbs.rs`) for
/// previewable files, icon glyph for everything else and while a
/// thumbnail is still being generated.
fn thumbnail<'a>(entry: &'a FileEntry, size: u16) -> Element<'a, Message> {
    if crate::thumbs::wants_thumbnail(&entry.mime_type) {
        if let Some(Some(path)) = crate::thumbs::get(&entry.path, &entry.mime_type) {
            return image(image::Handle::from_path(path))
                .width(size)
                .height(size)
                .content_fit(iced::ContentFit::Contain)
                .into();
        }
    }
    // Icon-glyph fallback (folders, and any file without a real
    // thumbnail): unlike the image path above, a bare `text()` widget has
    // no width/height of its own — at a large zoom level its glyph point
    // size (== `size`, up to 160) grew taller than the grid card itself,
    // pushing the filename below it clean out of the card's fixed height
    // and off-screen. A glyph reads fine well under the box size, so it's
    // capped, then centered in a box matching the image path's footprint
    // — same layout math for both branches regardless of zoom.
    let glyph_size = (size / 2).clamp(20, 64);
    container(text(entry.icon).font(ICON_FONT).size(glyph_size))
        .width(size)
        .height(size)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

pub fn view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    cursor: Option<usize>,
    view_mode: ViewMode,
    available_width: f32,
    sort_by: SortBy,
    sort_asc: bool,
    suppress_hover: bool,
    drop_hover: Option<&'a PathBuf>,
    zoom: u16,
    items_per_row_out: &'a std::cell::Cell<usize>,
) -> Element<'a, Message> {
    let content: Element<Message> = match view_mode {
        ViewMode::Grid => grid_view(entries, selected, cursor, available_width, suppress_hover, drop_hover, zoom, items_per_row_out),
        ViewMode::List => list_view(entries, selected, cursor, sort_by, sort_asc, suppress_hover, drop_hover),
    };

    let area = mouse_area(
        container(content)
            .width(Length::Fill)
            .padding(12)
            .style(theme::main_bg),
    )
    .on_right_press(Message::ContextMenuOpenBackground)
    .on_press(Message::BackgroundClicked);

    scrollable(area)
        .id(scrollable::Id::new("filelist"))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

const CARD_SPACING: f32 = 8.0;
const GRID_PADDING: f32 = 24.0; // matches the 12px padding on both sides in view()

fn grid_view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    cursor: Option<usize>,
    available_width: f32,
    suppress_hover: bool,
    drop_hover: Option<&'a PathBuf>,
    zoom: u16,
    items_per_row_out: &'a std::cell::Cell<usize>,
) -> Element<'a, Message> {
    if entries.is_empty() {
        return empty_message();
    }

    let card_width = zoom as f32 + 30.0;
    let usable = (available_width - GRID_PADDING).max(card_width);
    let items_per_row = (((usable + CARD_SPACING) / (card_width + CARD_SPACING)) as usize).max(1);
    items_per_row_out.set(items_per_row);

    let mut rows: Vec<Element<Message>> = Vec::new();
    let mut chunks = entries.chunks(items_per_row).enumerate();

    while let Some((row_i, chunk)) = chunks.next() {
        let mut row_items = row![].spacing(8);
        for (col_i, entry) in chunk.iter().enumerate() {
            let idx = row_i * items_per_row + col_i;
            let is_drop_target = entry.is_dir && drop_hover == Some(&entry.path);
            row_items = row_items.push(grid_card(
                entry,
                idx,
                selected.contains(&entry.path),
                cursor == Some(idx),
                suppress_hover,
                is_drop_target,
                card_width,
                zoom,
            ));
        }
        rows.push(row_items.into());
    }

    column(rows).spacing(8).into()
}

fn empty_message<'a>() -> Element<'a, Message> {
    container(text("Empty folder").style(theme::muted_text))
        .center_x(Length::Fill)
        .padding(40)
        .into()
}

fn grid_card<'a>(
    entry: &'a FileEntry,
    idx: usize,
    selected: bool,
    is_cursor: bool,
    suppress_hover: bool,
    is_drop_target: bool,
    card_width: f32,
    zoom: u16,
) -> Element<'a, Message> {
    let path = entry.path.clone();
    let name = if entry.name.chars().count() > 16 {
        format!("{}…", entry.name.chars().take(15).collect::<String>())
    } else {
        entry.name.clone()
    };
    let pal = theme::p();

    let border_color = if is_drop_target || selected { pal.accent } else if is_cursor { pal.muted } else { pal.border };
    let bg = if is_drop_target || selected { pal.selected_bg } else { pal.surface };
    let border_width = if is_drop_target || is_cursor { 2.0 } else if selected { 2.0 } else { 1.0 };

    let btn = button(
        column![
            container(thumbnail(entry, zoom)).width(Length::Fill).center_x(Length::Fill),
            text(name).size(11),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .width(card_width)
    .height(zoom as f32 + 40.0)
    .style(move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered if !suppress_hover => Color { r: bg.r + 0.05, g: bg.g + 0.05, b: bg.b + 0.08, a: 1.0 },
            _ => bg,
        })),
        text_color: pal.text,
        border: Border { color: border_color, width: border_width, radius: 6.0.into() },
        ..Default::default()
    });

    mouse_area(btn)
        .on_press(Message::EntryPressed(idx))
        .on_release(Message::EntryReleased(idx))
        .on_enter(Message::EntryHoverEnter(path.clone()))
        .on_exit(Message::EntryHoverExit(path))
        .on_right_press(Message::ContextMenuOpenEntry(idx))
        .into()
}

fn sort_header_btn<'a>(label: &'a str, by: SortBy, width: Length, sort_by: SortBy, sort_asc: bool) -> Element<'a, Message> {
    let is_active = sort_by == by;
    let text_label = if is_active {
        format!("{}  {}", label, if sort_asc { "\u{25b2}" } else { "\u{25bc}" })
    } else {
        label.to_string()
    };
    let pal = theme::p();

    button(text(text_label).size(12).style(move |_| iced::widget::text::Style {
        color: Some(if is_active { pal.text } else { pal.muted }),
    }))
    .on_press(Message::SortChanged(by))
    .width(width)
    .padding(0)
    .style(theme::flat_button)
    .into()
}

fn list_view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    cursor: Option<usize>,
    sort_by: SortBy,
    sort_asc: bool,
    suppress_hover: bool,
    drop_hover: Option<&'a PathBuf>,
) -> Element<'a, Message> {
    let header = row![
        text("").width(30),
        sort_header_btn("Name", SortBy::Name, Length::Fill, sort_by, sort_asc),
        sort_header_btn("Size", SortBy::Size, Length::Fixed(80.0), sort_by, sort_asc),
        sort_header_btn("Type", SortBy::Kind, Length::Fixed(120.0), sort_by, sort_asc),
        sort_header_btn("Modified", SortBy::Modified, Length::Fixed(150.0), sort_by, sort_asc),
    ]
    .spacing(8)
    .padding([4, 8])
    .align_y(iced::Alignment::Center);

    let mut col = column![header].spacing(1);

    if entries.is_empty() {
        col = col.push(container(text("Empty folder").style(theme::muted_text)).padding([20, 8]));
    }

    for (idx, entry) in entries.iter().enumerate() {
        let is_drop_target = entry.is_dir && drop_hover == Some(&entry.path);
        col = col.push(list_row(entry, idx, selected.contains(&entry.path), cursor == Some(idx), suppress_hover, is_drop_target));
    }

    col.into()
}

fn list_row<'a>(entry: &'a FileEntry, idx: usize, selected: bool, is_cursor: bool, suppress_hover: bool, is_drop_target: bool) -> Element<'a, Message> {
    let path = entry.path.clone();
    let pal = theme::p();
    let bg = if is_drop_target || selected { pal.selected_bg } else { pal.bg };
    let size_str = if entry.is_dir { "-".to_string() } else { format_size(entry.size) };
    let mime_short = if entry.is_dir {
        "Folder".to_string()
    } else {
        entry.mime_type.split('/').next_back().unwrap_or("file").to_string()
    };
    let modified = entry.modified.format("%Y-%m-%d %H:%M").to_string();

    let btn = button(
        row![
            container(thumbnail(entry, 18)).width(30),
            text(&entry.name).size(13).width(Length::Fill),
            text(size_str).size(12).width(80).style(theme::muted_text),
            text(mime_short).size(12).width(120).style(theme::muted_text),
            text(modified).size(12).width(150).style(theme::muted_text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .style(move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered if !suppress_hover => pal.surface,
            _ => bg,
        })),
        text_color: pal.text,
        border: Border {
            color: if is_drop_target || selected { pal.accent } else if is_cursor { pal.muted } else { Color::TRANSPARENT },
            width: if is_drop_target { 2.0 } else if selected || is_cursor { 1.0 } else { 0.0 },
            radius: 3.0.into(),
        },
        ..Default::default()
    });

    mouse_area(btn)
        .on_press(Message::EntryPressed(idx))
        .on_release(Message::EntryReleased(idx))
        .on_enter(Message::EntryHoverEnter(path.clone()))
        .on_exit(Message::EntryHoverExit(path))
        .on_right_press(Message::ContextMenuOpenEntry(idx))
        .into()
}
