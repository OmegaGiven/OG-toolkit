//! Native pavucontrol equivalent — all 5 of its tabs (Output/Input Devices,
//! Playback, Recording, Configuration), backed by `crate::audio`'s
//! `pactl -f json` parsing.

use iced::widget::{button, column, container, row, slider, text};
use iced::{Background, Border, Color, Length};
use std::collections::HashMap;

use crate::app::{AppColors, Message};
use crate::audio::{AudioCard, AudioDevice, AudioSnapshot, AudioStream, AudioTarget};
use crate::audio_meter::Meter;

/// Live peak meter bar — a thin filled track under a device's volume
/// slider. `pct` is 0.0-100.0 from `Meter::level_pct()`; `None` means no
/// meter is running for this device (wrong subtab, or `parec` failed to
/// spawn).
fn meter_bar<'a>(colors: AppColors, pct: Option<f32>) -> Element<'a> {
    let pct = pct.unwrap_or(0.0);
    // Green up to 70%, amber to 90%, red above — same "getting hot" ramp
    // as a VU meter, so clipping is obvious at a glance.
    let fill_color = if pct >= 90.0 {
        Color { r: 0.8, g: 0.15, b: 0.15, a: 1.0 }
    } else if pct >= 70.0 {
        Color { r: 0.85, g: 0.65, b: 0.1, a: 1.0 }
    } else {
        Color { r: 0.2, g: 0.7, b: 0.35, a: 1.0 }
    };

    // Tenths-of-a-percent FillPortion instead of whole percent — 100 steps
    // across a full-width bar was coarse enough to visibly step ("notch")
    // as the level moved; 1000 steps reads as continuous motion.
    let tenths = (pct * 10.0).round().clamp(0.0, 1000.0) as u16;

    // Fill gets its own radius (not just the outer track) so both its
    // ends — leading and trailing — read as rounded caps at any fill
    // level, instead of a square-cut rectangle sitting inside a rounded
    // track.
    let filled = container(text("").size(1)).width(Length::FillPortion(tenths.max(1))).height(Length::Fixed(6.0)).style(move |_| container::Style {
        background: Some(Background::Color(fill_color)),
        border: Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    });
    let empty = container(text("").size(1)).width(Length::FillPortion(1000 - tenths));

    container(row![filled, empty].height(Length::Fixed(6.0)))
        .width(Length::Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(Background::Color(colors.surface)),
            border: Border { color: colors.border, width: 1.0, radius: 3.0.into() },
            ..Default::default()
        })
        .into()
}

/// Same danger-red already used for "Remove section" etc. elsewhere in this
/// app — reused here so a muted device reads as unambiguously "off" instead
/// of blending into the rest of the card.
const MUTE_RED: Color = Color { r: 0.6, g: 0.1, b: 0.1, a: 1.0 };

fn nerd_font() -> iced::Font {
    iced::Font::with_name("Symbols Nerd Font")
}

/// Single click toggles mute — same glyphs og-bar's own volume module uses
/// (`\u{f0581}` muted / `\u{f057e}` unmuted) so the two apps read as one
/// system instead of inventing a second icon language.
fn mute_button<'a>(colors: AppColors, muted: bool, target: AudioTarget) -> Element<'a> {
    let icon = if muted { "\u{f0581}" } else { "\u{f057e}" };
    let fg = if muted { Color::WHITE } else { colors.text };
    button(text(icon).size(16).font(nerd_font()).style(move |_| text::Style { color: Some(fg) }))
        .padding([6, 10])
        .style(move |_, status| iced::widget::button::Style {
            background: Some(Background::Color(if muted {
                MUTE_RED
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                colors.surface
            } else {
                Color::TRANSPARENT
            })),
            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
            text_color: fg,
            ..Default::default()
        })
        .on_press(Message::AudioMuteToggled(target, !muted))
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSubTab {
    Output,
    Input,
    Playback,
    Recording,
    Configuration,
}

impl AudioSubTab {
    pub const ALL: [AudioSubTab; 5] =
        [AudioSubTab::Output, AudioSubTab::Input, AudioSubTab::Playback, AudioSubTab::Recording, AudioSubTab::Configuration];

    pub fn label(&self) -> &'static str {
        match self {
            AudioSubTab::Output => "Output Devices",
            AudioSubTab::Input => "Input Devices",
            AudioSubTab::Playback => "Playback",
            AudioSubTab::Recording => "Recording",
            AudioSubTab::Configuration => "Configuration",
        }
    }
}

fn device_card<'a>(colors: AppColors, dev: &'a AudioDevice, target: AudioTarget, meter_pct: Option<f32>) -> Element<'a> {
    let card_style = move |_: &_| container::Style {
        background: Some(Background::Color(colors.sec_bg)),
        border: Border { color: if dev.is_default { colors.accent } else { colors.border }, width: if dev.is_default { 2.0 } else { 1.0 }, radius: colors.radius.into() },
        ..Default::default()
    };

    let default_btn: Element<'a> = if dev.is_default {
        text("Default").size(11).style(move |_| text::Style { color: Some(colors.accent) }).into()
    } else {
        button(text("Set as default").size(11).style(move |_| text::Style { color: Some(colors.text) }))
            .style(move |_, status| iced::widget::button::Style {
                background: Some(Background::Color(if matches!(status, iced::widget::button::Status::Hovered) { colors.accent } else { colors.surface })),
                text_color: colors.text,
                border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                ..Default::default()
            })
            .on_press(Message::AudioSetDefault(target.clone()))
            .into()
    };

    let mute_target = target.clone();
    let vol_target = target;

    container(
        column![
            row![
                text(dev.description.clone()).size(14).style(move |_| text::Style { color: Some(colors.text) }),
                iced::widget::horizontal_space(),
                default_btn,
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
            row![
                mute_button(colors, dev.mute, mute_target),
                slider(0..=150u32, dev.volume_pct, move |v| Message::AudioVolumeChanged(vol_target.clone(), v)).step(1u32),
                text(format!("{}%", dev.volume_pct)).size(12).style(move |_| text::Style { color: Some(colors.dim_text) }),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10),
            meter_bar(colors, meter_pct),
        ]
        .spacing(6)
        .padding(12),
    )
    .style(card_style)
    .width(Length::Fill)
    .into()
}

fn stream_card<'a>(colors: AppColors, s: &'a AudioStream, target: AudioTarget) -> Element<'a> {
    let card_style = move |_: &_| container::Style {
        background: Some(Background::Color(colors.sec_bg)),
        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    };
    let mute_target = target.clone();
    let vol_target = target;

    container(
        column![
            text(s.app_name.clone()).size(14).style(move |_| text::Style { color: Some(colors.text) }),
            row![
                mute_button(colors, s.mute, mute_target),
                slider(0..=150u32, s.volume_pct, move |v| Message::AudioVolumeChanged(vol_target.clone(), v)).step(1u32),
                text(format!("{}%", s.volume_pct)).size(12).style(move |_| text::Style { color: Some(colors.dim_text) }),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10),
        ]
        .spacing(6)
        .padding(12),
    )
    .style(card_style)
    .width(Length::Fill)
    .into()
}

fn card_config<'a>(colors: AppColors, card: &'a AudioCard) -> Element<'a> {
    let card_style = move |_: &_| container::Style {
        background: Some(Background::Color(colors.sec_bg)),
        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    };

    let profile_rows: Vec<Element<'a>> = card
        .profiles
        .iter()
        .filter(|p| p.available)
        .map(|p| {
            let active = p.id == card.active_profile;
            let card_name = card.name.clone();
            let profile_id = p.id.clone();
            button(text(p.description.clone()).size(12).style(move |_| text::Style { color: Some(if active { colors.bar_bg } else { colors.text }) }))
                .width(Length::Fill)
                .padding([6, 10])
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(Background::Color(if active { colors.accent } else { iced::Color::TRANSPARENT })),
                    text_color: if active { colors.bar_bg } else { colors.text },
                    border: Border { radius: colors.radius.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(Message::AudioProfileSelected(card_name.clone(), profile_id.clone()))
                .into()
        })
        .collect();

    container(
        column![
            text(card.description.clone()).size(14).style(move |_| text::Style { color: Some(colors.text) }),
            column(profile_rows).spacing(2),
        ]
        .spacing(8)
        .padding(12),
    )
    .style(card_style)
    .width(Length::Fill)
    .into()
}

type Element<'a> = iced::Element<'a, Message>;

pub fn view<'a>(colors: AppColors, subtab: AudioSubTab, snapshot: &'a AudioSnapshot, meters: &'a HashMap<String, Meter>) -> Element<'a> {
    let tab_row: Vec<Element> = AudioSubTab::ALL
        .iter()
        .map(|t| {
            let active = *t == subtab;
            let t = *t;
            button(text(t.label()).size(13).style(move |_| text::Style { color: Some(if active { colors.bar_bg } else { colors.text }) }))
                .padding([8, 14])
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(Background::Color(if active { colors.accent } else { colors.surface })),
                    text_color: if active { colors.bar_bg } else { colors.text },
                    border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                    ..Default::default()
                })
                .on_press(Message::AudioSubTabSelected(t))
                .into()
        })
        .collect();

    let empty = |label: &'static str| -> Element<'static> {
        text(label).size(12).style(move |_| text::Style { color: Some(colors.dim_text) }).into()
    };

    let content: Element = match subtab {
        AudioSubTab::Output => {
            if snapshot.sinks.is_empty() {
                empty("No output devices found.")
            } else {
                column(snapshot.sinks.iter().map(|d| {
                    let pct = meters.get(&d.name).map(Meter::level_pct);
                    device_card(colors, d, AudioTarget::Sink(d.name.clone()), pct)
                }))
                .spacing(10)
                .into()
            }
        }
        AudioSubTab::Input => {
            if snapshot.sources.is_empty() {
                empty("No input devices found.")
            } else {
                column(snapshot.sources.iter().map(|d| {
                    let pct = meters.get(&d.name).map(Meter::level_pct);
                    device_card(colors, d, AudioTarget::Source(d.name.clone()), pct)
                }))
                .spacing(10)
                .into()
            }
        }
        AudioSubTab::Playback => {
            if snapshot.sink_inputs.is_empty() {
                empty("Nothing is currently playing audio.")
            } else {
                column(snapshot.sink_inputs.iter().map(|s| stream_card(colors, s, AudioTarget::SinkInput(s.index)))).spacing(10).into()
            }
        }
        AudioSubTab::Recording => {
            if snapshot.source_outputs.is_empty() {
                empty("Nothing is currently recording audio.")
            } else {
                column(snapshot.source_outputs.iter().map(|s| stream_card(colors, s, AudioTarget::SourceOutput(s.index)))).spacing(10).into()
            }
        }
        AudioSubTab::Configuration => {
            if snapshot.cards.is_empty() {
                empty("No audio cards found.")
            } else {
                column(snapshot.cards.iter().map(|c| card_config(colors, c))).spacing(10).into()
            }
        }
    };

    column![
        row(tab_row).spacing(6),
        iced::widget::scrollable(content).height(Length::Fill),
    ]
    .spacing(16)
    .padding(20)
    .into()
}
