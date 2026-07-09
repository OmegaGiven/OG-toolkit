//! Native pavucontrol equivalent — all 5 of its tabs (Output/Input Devices,
//! Playback, Recording, Configuration), backed by `crate::audio`'s
//! `pactl -f json` parsing.

use iced::widget::{button, column, container, row, slider, text, toggler};
use iced::{Background, Border, Length};

use crate::app::{AppColors, Message};
use crate::audio::{AudioCard, AudioDevice, AudioSnapshot, AudioStream, AudioTarget};

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

fn device_card<'a>(colors: AppColors, dev: &'a AudioDevice, target: AudioTarget) -> Element<'a> {
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
                toggler(dev.mute).label("Muted").on_toggle(move |v| Message::AudioMuteToggled(mute_target.clone(), v)),
                slider(0..=150u32, dev.volume_pct, move |v| Message::AudioVolumeChanged(vol_target.clone(), v)).step(1u32),
                text(format!("{}%", dev.volume_pct)).size(12).style(move |_| text::Style { color: Some(colors.dim_text) }),
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
                toggler(s.mute).label("Muted").on_toggle(move |v| Message::AudioMuteToggled(mute_target.clone(), v)),
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

pub fn view<'a>(colors: AppColors, subtab: AudioSubTab, snapshot: &'a AudioSnapshot) -> Element<'a> {
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
                column(snapshot.sinks.iter().map(|d| device_card(colors, d, AudioTarget::Sink(d.name.clone())))).spacing(10).into()
            }
        }
        AudioSubTab::Input => {
            if snapshot.sources.is_empty() {
                empty("No input devices found.")
            } else {
                column(snapshot.sources.iter().map(|d| device_card(colors, d, AudioTarget::Source(d.name.clone())))).spacing(10).into()
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
