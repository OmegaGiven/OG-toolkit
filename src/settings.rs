//! The popout customization menu (PLAN.md section 7) — the actual point of
//! this rebuild. Item-size/spacing/padding/module-enable changes apply live
//! (the popup reads straight from `BarConfig`, same as the bar itself).
//! Edge/thickness changes need a real wlr layer-shell relayout, which this
//! process can't do to its own already-mapped surface — "Apply & Save"
//! writes the config and respawns the bar to pick up the new anchor/size,
//! matching the plan's own carve-out for changes that need a full relayout.

use iced::widget::{button, checkbox, column, container, row, slider, text};
use iced::{Background, Border, Color, Element, Length};

use og_config::{Edge, ModuleKind};

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Start,
    Center,
    End,
}

pub struct SettingsButton;

impl Module for SettingsButton {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let fg = colors.text;
        button(
            container(text("").size(16).style(move |_| text::Style { color: Some(fg) }))
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
        .on_press(Message::OpenSettingsPopup)
        .into()
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
        .on_press(Message::SetEdge(edge))
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
        slider(range, value, on_change).step(1u32),
    ]
    .spacing(2)
    .into()
}

fn module_toggle_row(_colors: AppColors, section: Section, index: usize, kind: &ModuleKind, enabled: bool) -> Element<'static, Message> {
    let label = kind_label(kind);
    row![checkbox(label, enabled).on_toggle(move |_| Message::ToggleModule(section, index))].into()
}

pub fn popup_view(colors: AppColors, bar_cfg: &og_config::BarConfig) -> Element<'_, Message> {
    let edge_row = row![
        edge_button(colors, "Top", Edge::Top, bar_cfg.position),
        edge_button(colors, "Bottom", Edge::Bottom, bar_cfg.position),
        edge_button(colors, "Left", Edge::Left, bar_cfg.position),
        edge_button(colors, "Right", Edge::Right, bar_cfg.position),
    ]
    .spacing(4);

    let sliders = column![
        labeled_slider(colors, "Thickness", bar_cfg.thickness, 16..=96, Message::SetThickness),
        labeled_slider(colors, "Item size", bar_cfg.item_size, 16..=80, Message::SetItemSize),
        labeled_slider(colors, "Spacing", bar_cfg.spacing, 0..=32, Message::SetSpacing),
        labeled_slider(colors, "Padding", bar_cfg.padding, 0..=32, Message::SetPadding),
    ]
    .spacing(8);

    let module_list = column(
        [
            (Section::Start, &bar_cfg.modules_start),
            (Section::Center, &bar_cfg.modules_center),
            (Section::End, &bar_cfg.modules_end),
        ]
        .into_iter()
        .flat_map(|(section, list)| {
            list.iter()
                .enumerate()
                .map(move |(i, m)| module_toggle_row(colors, section, i, &m.kind, m.enabled))
        }),
    )
    .spacing(4);

    let apply_button = button(text("Apply relayout (Edge/Thickness)").size(12))
        .padding(6)
        .style(move |_, status| button::Style {
            background: Some(Background::Color(if matches!(status, button::Status::Hovered) {
                colors.header_btn_bg
            } else {
                colors.surface
            })),
            border: Border { radius: colors.radius.into(), ..Default::default() },
            text_color: colors.text,
            ..Default::default()
        })
        .on_press(Message::ApplyRelayout);

    container(
        column![
            text("Bar edge").size(12).style(move |_| text::Style { color: Some(colors.dim_text) }),
            edge_row,
            sliders,
            text("Modules").size(12).style(move |_| text::Style { color: Some(colors.dim_text) }),
            module_list,
            apply_button,
        ]
        .spacing(10)
        .padding(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: Some(colors.bg_fill),
        border: Border { color: colors.accent, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    })
    .into()
}
