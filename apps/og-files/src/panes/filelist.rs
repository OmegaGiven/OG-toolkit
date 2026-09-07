use iced::{
    widget::{button, column, container, image, mouse_area, row, scrollable, text},
    Background, Border, Color, Element, Length,
};
use std::collections::HashSet;
use std::path::PathBuf;
use crate::app::{Message, SortBy, ViewMode, ACCENT, BG, TEXT, MUTED, SELECTED_BG, SURFACE};
use crate::filesystem::{FileEntry, format_size};

const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

/// Real decoded thumbnail for image files, icon glyph for everything else.
fn thumbnail<'a>(entry: &'a FileEntry, size: u16) -> Element<'a, Message> {
    if entry.previewable {
        image(image::Handle::from_path(&entry.path))
            .width(size)
            .height(size)
            .content_fit(iced::ContentFit::Contain)
            .into()
    } else {
        text(entry.icon).font(ICON_FONT).size(size).into()
    }
}

pub fn view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    view_mode: &ViewMode,
    available_width: f32,
    sort_by: &SortBy,
    sort_asc: bool,
    suppress_hover: bool,
    drop_hover: Option<&'a PathBuf>,
) -> Element<'a, Message> {
    let content: Element<Message> = match view_mode {
        ViewMode::Grid => grid_view(entries, selected, available_width, suppress_hover, drop_hover),
        ViewMode::List => list_view(entries, selected, sort_by, sort_asc, suppress_hover, drop_hover),
    };

    let area = mouse_area(
        container(content)
            .width(Length::Fill)
            .padding(12)
            .style(|_| container::Style {
                background: Some(Background::Color(BG())),
                ..Default::default()
            })
    )
    .on_right_press(Message::ContextMenuOpenBackground);

    scrollable(area)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

const CARD_WIDTH: f32 = 110.0;
const CARD_SPACING: f32 = 8.0;
const GRID_PADDING: f32 = 24.0; // matches the 12px padding on both sides in view()

fn grid_view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    available_width: f32,
    suppress_hover: bool,
    drop_hover: Option<&'a PathBuf>,
) -> Element<'a, Message> {
    if entries.is_empty() {
        return container(
            text("Empty folder").style(move |_| text::Style { color: Some(MUTED()) })
        )
        .center_x(Length::Fill)
        .padding(40)
        .into();
    }

    let usable = (available_width - GRID_PADDING).max(CARD_WIDTH);
    let items_per_row = (((usable + CARD_SPACING) / (CARD_WIDTH + CARD_SPACING)) as usize).max(1);
    let mut rows: Vec<Element<Message>> = Vec::new();
    let mut chunks = entries.chunks(items_per_row).peekable();

    while let Some(chunk) = chunks.next() {
        let mut row_items = row![].spacing(8);
        for entry in chunk {
            let is_drop_target = entry.is_dir && drop_hover == Some(&entry.path);
            row_items = row_items.push(grid_card(entry, selected.contains(&entry.path), suppress_hover, is_drop_target));
        }
        rows.push(row_items.into());
    }

    column(rows).spacing(8).into()
}

fn grid_card<'a>(entry: &'a FileEntry, selected: bool, suppress_hover: bool, is_drop_target: bool) -> Element<'a, Message> {
    let path = entry.path.clone();
    let path2 = entry.path.clone();
    let path3 = entry.path.clone();
    let path4 = entry.path.clone();
    let path5 = entry.path.clone();
    let name = if entry.name.len() > 14 {
        format!("{}…", &entry.name[..13])
    } else {
        entry.name.clone()
    };

    let border_color = if is_drop_target { ACCENT() } else if selected { ACCENT() } else { Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 } };
    let bg = if is_drop_target { SELECTED_BG() } else if selected { SELECTED_BG() } else { SURFACE() };

    let btn = button(
        column![
            container(thumbnail(entry, 40)).width(Length::Fill).center_x(Length::Fill),
            text(name).size(11).style(move |_| text::Style { color: Some(TEXT()) }),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill)
    )
    .width(110)
    .height(90)
    .style(move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered if !suppress_hover => Color { r: bg.r + 0.05, g: bg.g + 0.05, b: bg.b + 0.08, a: 1.0 },
            _ => bg,
        })),
        text_color: TEXT(),
        border: Border { color: border_color, width: if is_drop_target { 2.0 } else if selected { 2.0 } else { 1.0 }, radius: 6.0.into() },
        ..Default::default()
    });

    mouse_area(btn)
        .on_press(Message::EntryPressed(path))
        .on_release(Message::EntryReleased(path4))
        .on_enter(Message::EntryHoverEnter(path3))
        .on_exit(Message::EntryHoverExit(path5))
        .on_right_press(Message::ContextMenuOpenEntry(path2))
        .into()
}

fn sort_header_btn<'a>(label: &'a str, by: SortBy, width: Length, sort_by: &SortBy, sort_asc: bool) -> Element<'a, Message> {
    let is_active = *sort_by == by;
    let text_label = if is_active {
        format!("{}  {}", label, if sort_asc { "\u{25b2}" } else { "\u{25bc}" })
    } else {
        label.to_string()
    };

    button(text(text_label).size(12).style(move |_| text::Style {
        color: Some(if is_active { TEXT() } else { MUTED() }),
    }))
    .on_press(Message::SortChanged(by))
    .width(width)
    .padding(0)
    .style(|_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => SURFACE(),
            _ => BG(),
        })),
        text_color: TEXT(),
        border: Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    })
    .into()
}

fn list_view<'a>(
    entries: &'a [FileEntry],
    selected: &'a HashSet<PathBuf>,
    sort_by: &SortBy,
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
        col = col.push(
            container(text("Empty folder").style(move |_| text::Style { color: Some(MUTED()) }))
                .padding([20, 8])
        );
    }

    for entry in entries {
        let is_drop_target = entry.is_dir && drop_hover == Some(&entry.path);
        col = col.push(list_row(entry, selected.contains(&entry.path), suppress_hover, is_drop_target));
    }

    col.into()
}

fn list_row<'a>(entry: &'a FileEntry, selected: bool, suppress_hover: bool, is_drop_target: bool) -> Element<'a, Message> {
    let path = entry.path.clone();
    let path2 = entry.path.clone();
    let path3 = entry.path.clone();
    let path4 = entry.path.clone();
    let path5 = entry.path.clone();
    let bg = if is_drop_target { SELECTED_BG() } else if selected { SELECTED_BG() } else { BG() };
    let size_str = if entry.is_dir { "-".to_string() } else { format_size(entry.size) };
    let mime_short = if entry.is_dir {
        "Folder".to_string()
    } else {
        entry.mime_type.split('/').last().unwrap_or("file").to_string()
    };
    let modified = entry.modified.format("%Y-%m-%d %H:%M").to_string();

    let btn = button(
        row![
            container(thumbnail(entry, 18)).width(30),
            text(&entry.name).size(13).width(Length::Fill),
            text(size_str).size(12).width(80).style(move |_| text::Style { color: Some(MUTED()) }),
            text(mime_short).size(12).width(120).style(move |_| text::Style { color: Some(MUTED()) }),
            text(modified).size(12).width(150).style(move |_| text::Style { color: Some(MUTED()) }),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    )
    .width(Length::Fill)
    .style(move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered if !suppress_hover => SURFACE(),
            _ => bg,
        })),
        text_color: TEXT(),
        border: Border {
            color: if is_drop_target || selected { ACCENT() } else { Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } },
            width: if is_drop_target { 2.0 } else if selected { 1.0 } else { 0.0 },
            radius: 3.0.into(),
        },
        ..Default::default()
    });

    mouse_area(btn)
        .on_press(Message::EntryPressed(path))
        .on_release(Message::EntryReleased(path4))
        .on_enter(Message::EntryHoverEnter(path3))
        .on_exit(Message::EntryHoverExit(path5))
        .on_right_press(Message::ContextMenuOpenEntry(path2))
        .into()
}
