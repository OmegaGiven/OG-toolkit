//! og-bar's full-size config editor (PLAN.md section 8 in og-bar's own
//! PLAN.md at ~/.local/src/og-bar/PLAN.md). Same `BarConfig` fields and
//! the same edge-picker/sliders/module-toggle-list shape as og-bar's own
//! popout menu (its `src/settings.rs`) — kept structurally identical on
//! purpose so the two editors can't drift into disagreeing about what a
//! field means, even though they're separate binaries and can't literally
//! share one view function today. Unlike the popout (which auto-saves
//! every change instantly), this tab follows the rest of og-settings:
//! changes stage in-memory and only hit disk on "Apply & Save".

use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Background, Border, Color, Element, Length};

use og_config::{BarConfig, Edge, ModuleConfig, ModuleKind};

use crate::app::{AppColors, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarSection {
    Start,
    Center,
    End,
}

fn section_list(bar_config: &mut BarConfig, section: BarSection) -> &mut Vec<ModuleConfig> {
    match section {
        BarSection::Start => &mut bar_config.modules_start,
        BarSection::Center => &mut bar_config.modules_center,
        BarSection::End => &mut bar_config.modules_end,
    }
}

pub fn toggle_module(bar_config: &mut BarConfig, section: BarSection, index: usize) {
    if let Some(m) = section_list(bar_config, section).get_mut(index) {
        m.enabled = !m.enabled;
    }
}

pub fn set_clock_timezone(bar_config: &mut BarConfig, section: BarSection, index: usize, tz: String) {
    if let Some(m) = section_list(bar_config, section).get_mut(index) {
        if let ModuleKind::Clock { timezone } = &mut m.kind {
            *timezone = tz;
        }
    }
}

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
        ModuleKind::Taskbar => "Taskbar".to_string(),
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

fn module_toggle_row(section: BarSection, index: usize, kind: &ModuleKind, enabled: bool) -> Element<'static, Message> {
    let label = kind_label(kind);
    let mut r = row![checkbox(label, enabled).on_toggle(move |_| Message::BarToggleModule(section, index))].spacing(8);
    if let ModuleKind::Clock { timezone } = kind {
        r = r.push(
            text_input("IANA tz, e.g. America/Chicago (blank = local)", timezone)
                .size(11)
                .width(Length::Fixed(200.0))
                .on_input(move |v| Message::BarSetClockTimezone(section, index, v)),
        );
    }
    r.into()
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

    let module_section = |title: &'static str, section: BarSection, list: &'a [ModuleConfig]| -> Element<'a, Message> {
        column![
            text(title).size(12).style(move |_| text::Style { color: Some(Color { a: 0.6, ..colors.text }) }),
            column(
                list.iter()
                    .enumerate()
                    .map(|(i, m)| module_toggle_row(section, i, &m.kind, m.enabled))
            )
            .spacing(4),
        ]
        .spacing(4)
        .into()
    };

    let module_lists = row![
        module_section("Start", BarSection::Start, &bar_config.modules_start),
        module_section("Center", BarSection::Center, &bar_config.modules_center),
        module_section("End", BarSection::End, &bar_config.modules_end),
    ]
    .spacing(24);

    container(
        column![
            text("Bar edge").size(13).style(move |_| text::Style { color: Some(Color { a: 0.7, ..colors.text }) }),
            edge_row,
            sliders,
            text("Modules").size(13).style(move |_| text::Style { color: Some(Color { a: 0.7, ..colors.text }) }),
            module_lists,
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
