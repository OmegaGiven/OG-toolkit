//! og-bar's full-size config editor (PLAN.md section 8 in og-bar's own
//! PLAN.md at ~/.local/src/og-bar/PLAN.md). Structurally mirrors og-bar's
//! own popout (its `src/settings.rs`) on purpose, including the
//! section system (percent share + Start/Middle/End alignment, which
//! reads as Left/Middle/Right or Top/Middle/Bottom depending on the
//! bar's edge) — kept in sync by hand since they're separate binaries.
//! Unlike the popout (which auto-saves every change instantly), this tab
//! follows the rest of og-settings: changes stage in-memory and only hit
//! disk on "Apply & Save".

use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Background, Border, Color, Element, Length};

use og_config::{BarConfig, BarSection, Edge, ModuleKind, SectionAlign};

use crate::app::{AppColors, Message};

fn kind_label(kind: &ModuleKind) -> String {
    match kind {
        ModuleKind::Workspaces => "Workspaces".to_string(),
        ModuleKind::Clock { .. } => "Clock".to_string(),
        ModuleKind::Cpu => "CPU".to_string(),
        ModuleKind::Memory => "Memory".to_string(),
        ModuleKind::Tray => "Tray".to_string(),
        ModuleKind::Bluetooth => "Bluetooth".to_string(),
        ModuleKind::Network => "Network".to_string(),
        ModuleKind::Pulseaudio => "Volume".to_string(),
        ModuleKind::Launcher { tooltip, .. } => tooltip.clone(),
        ModuleKind::Settings => "Settings".to_string(),
        ModuleKind::Power => "Power".to_string(),
    }
}

fn edge_button(colors: AppColors, label: &'static str, edge: Edge, current: Edge) -> Element<'static, Message> {
    let selected = edge == current;
    let fg = if selected { colors.bar_bg } else { colors.text };
    let bg = if selected { colors.accent } else { Color::TRANSPARENT };
    button(text(label).size(13).style(move |_| text::Style { color: Some(fg) }))
        .padding(6)
        .style(move |_, _| button::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
            text_color: fg,
            ..Default::default()
        })
        .on_press(Message::BarSetEdge(edge))
        .into()
}

fn align_button(colors: AppColors, label: &'static str, align: SectionAlign, current: SectionAlign, section: usize) -> Element<'static, Message> {
    let selected = align == current;
    let fg = if selected { colors.bar_bg } else { colors.text };
    let bg = if selected { colors.accent } else { Color::TRANSPARENT };
    button(text(label).size(11).style(move |_| text::Style { color: Some(fg) }))
        .padding([3, 8])
        .style(move |_, _| button::Style {
            background: Some(Background::Color(bg)),
            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
            text_color: fg,
            ..Default::default()
        })
        .on_press(Message::BarSetSectionAlign(section, align))
        .into()
}

fn labeled_slider(
    colors: AppColors,
    label: &'static str,
    value: u32,
    range: std::ops::RangeInclusive<u32>,
    on_change: impl Fn(u32) -> Message + 'static,
) -> Element<'static, Message> {
    column![
        text(format!("{label}: {value}")).size(12).style(move |_| text::Style { color: Some(colors.text) }),
        iced::widget::slider(range, value, on_change).step(1u32),
    ]
    .spacing(2)
    .into()
}

fn module_toggle_row(colors: AppColors, section: usize, index: usize, kind: &ModuleKind, enabled: bool) -> Element<'static, Message> {
    let label = kind_label(kind);
    let checkbox_widget = checkbox(label, enabled)
        .on_toggle(move |_| Message::BarToggleModule(section, index))
        .style(move |_, _| iced::widget::checkbox::Style {
            background: Background::Color(colors.surface),
            icon_color: colors.accent,
            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
            text_color: Some(colors.text),
        });
    let mut r = row![checkbox_widget].spacing(8);
    if let ModuleKind::Clock { timezone } = kind {
        r = r.push(
            text_input("IANA tz, e.g. America/Chicago (blank = local)", timezone)
                .size(11)
                .width(Length::Fixed(200.0))
                .on_input(move |v| Message::BarSetClockTimezone(section, index, v)),
        );
    }
    r = r.push(iced::widget::horizontal_space());
    r = r.push(
        button(text("Remove").size(11).style(move |_| text::Style { color: Some(colors.text) }))
            .padding(4)
            .style(move |_, status| button::Style {
                background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                    colors.header_btn_bg
                } else {
                    Color::TRANSPARENT
                })),
                border: Border { radius: colors.radius.into(), ..Default::default() },
                text_color: colors.text,
                ..Default::default()
            })
            .on_press(Message::BarRemoveModule(section, index)),
    );
    r.into()
}

fn add_module_row(colors: AppColors, section: usize, present: &[ModuleKind]) -> Element<'static, Message> {
    let addable: Vec<Element<Message>> = ModuleKind::addable()
        .iter()
        .filter(|k| !present.contains(k))
        .map(|kind| {
            let kind = kind.clone();
            let label = kind_label(&kind);
            button(text(format!("+ {label}")).size(11).style(move |_| text::Style { color: Some(colors.text) }))
                .padding([2, 6])
                .style(move |_, status| button::Style {
                    background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                        colors.accent
                    } else {
                        colors.surface
                    })),
                    border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                    text_color: colors.text,
                    ..Default::default()
                })
                .on_press(Message::BarAddModule(section, kind))
                .into()
        })
        .collect();
    if addable.is_empty() {
        return iced::widget::Space::new(Length::Shrink, Length::Shrink).into();
    }
    row(addable).spacing(4).wrap().into()
}

fn section_block(colors: AppColors, position: Edge, index: usize, sec: &BarSection) -> Element<'static, Message> {
    let present: Vec<ModuleKind> = sec.modules.iter().map(|m| m.kind.clone()).collect();
    let rows: Vec<Element<Message>> = sec
        .modules
        .iter()
        .enumerate()
        .map(|(i, m)| module_toggle_row(colors, index, i, &m.kind, m.enabled))
        .collect();

    let align_row = row![
        align_button(colors, SectionAlign::Start.label(position), SectionAlign::Start, sec.align, index),
        align_button(colors, SectionAlign::Middle.label(position), SectionAlign::Middle, sec.align, index),
        align_button(colors, SectionAlign::End.label(position), SectionAlign::End, sec.align, index),
    ]
    .spacing(4);

    let remove_section_btn = button(text("Remove section").size(11).style(move |_| text::Style { color: Some(colors.text) }))
        .padding([2, 6])
        .style(move |_, _| button::Style {
            background: Some(Background::Color(Color { r: 0.6, g: 0.1, b: 0.1, a: 1.0 })),
            text_color: Color::WHITE,
            border: Border { radius: colors.radius.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::BarRemoveSection(index));

    container(
        column![
            row![
                text(format!("Section {}", index + 1)).size(12).style(move |_| text::Style { color: Some(Color { a: 0.7, ..colors.text }) }),
                iced::widget::horizontal_space(),
                remove_section_btn,
            ]
            .align_y(iced::Alignment::Center),
            labeled_slider(colors, "Share", sec.percent, 5..=100, move |v| Message::BarSetSectionPercent(index, v)),
            align_row,
            column(rows).spacing(4),
            add_module_row(colors, index, &present),
        ]
        .spacing(8)
        .padding(10),
    )
    .style(move |_| container::Style {
        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

pub fn view<'a>(bar_config: &'a BarConfig, colors: AppColors) -> Element<'a, Message> {
    let edge_row = row![
        edge_button(colors, "Top", Edge::Top, bar_config.position),
        edge_button(colors, "Bottom", Edge::Bottom, bar_config.position),
        edge_button(colors, "Left", Edge::Left, bar_config.position),
        edge_button(colors, "Right", Edge::Right, bar_config.position),
    ]
    .spacing(6);

    let sliders = column![
        labeled_slider(colors, "Thickness", bar_config.thickness, 16..=96, Message::BarSetThickness),
        labeled_slider(colors, "Item size", bar_config.item_size, 16..=80, Message::BarSetItemSize),
        labeled_slider(colors, "Spacing", bar_config.spacing, 0..=32, Message::BarSetSpacing),
        labeled_slider(colors, "Padding", bar_config.padding, 0..=32, Message::BarSetPadding),
    ]
    .spacing(10);

    let add_section_btn = button(text("+ Add section").size(12).style(move |_| text::Style { color: Some(colors.text) }))
        .padding([4, 10])
        .style(move |_, status| button::Style {
            background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                colors.accent
            } else {
                colors.surface
            })),
            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
            text_color: colors.text,
            ..Default::default()
        })
        .on_press(Message::BarAddSection);

    let sections = column(
        bar_config
            .sections
            .iter()
            .enumerate()
            .map(|(i, s)| section_block(colors, bar_config.position, i, s))
            .chain(std::iter::once(add_section_btn.into())),
    )
    .spacing(12);

    container(
        column![
            text("Bar edge").size(13).style(move |_| text::Style { color: Some(Color { a: 0.7, ..colors.text }) }),
            edge_row,
            sliders,
            text("Sections").size(13).style(move |_| text::Style { color: Some(Color { a: 0.7, ..colors.text }) }),
            text("Each section gets a share of the bar's length (not required to sum to 100 — it's a relative split) and packs its modules toward the start, middle, or end of that share.")
                .size(11)
                .style(move |_| text::Style { color: Some(Color { a: 0.5, ..colors.text }) }),
            sections,
            text("og-bar doesn't watch this file yet (PLAN.md section 2) — restart og-bar after Apply & Save to see any change here, not just edge/thickness.")
                .size(11)
                .style(move |_| text::Style { color: Some(Color { a: 0.5, ..colors.text }) }),
        ]
        .spacing(16)
        .padding(4),
    )
    .width(Length::Fill)
    .into()
}
