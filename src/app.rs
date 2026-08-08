use iced::widget::{column, container, row, scrollable, text};
use iced::{Background, Border, Color, Element, Event, Length, Task};
use iced::keyboard;
use iced::window;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::config::{Config, VoiceConfig, APP_TINT_SEED};
use crate::pipeline::{self, SharedState, Stage};
use crate::placement;
use og_theme::AppColors;

pub const SIZE: (f32, f32) = (380.0, 300.0);

/// Auto-close this long after the answer is shown, if the user hasn't
/// already dismissed it (Escape / click-away) — the "idle time" half of
/// the close condition the brief asks for.
const IDLE_CLOSE_AFTER_DONE: std::time::Duration = std::time::Duration::from_secs(25);

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Close,
    Positioned,
    Reveal(window::Id),
}

pub struct App {
    config: Config,
    #[allow(dead_code)]
    voice_config: VoiceConfig,
    shared: SharedState,
    #[allow(dead_code)]
    stop_flag: Arc<AtomicBool>,
    stage: Stage,
    transcript: String,
    answer: String,
    done_since: Option<Instant>,
}

impl App {
    pub fn new(shared: SharedState, stop_flag: Arc<AtomicBool>) -> (Self, Task<Message>) {
        (
            Self {
                config: Config::load(),
                voice_config: VoiceConfig::load(),
                shared,
                stop_flag,
                stage: Stage::StartingStt,
                transcript: String::new(),
                answer: String::new(),
                done_since: None,
            },
            Task::perform(async { placement::move_to_corner(SIZE) }, |_| Message::Positioned)
                .chain(window::get_latest().map(|id| match id {
                    Some(id) => Message::Reveal(id),
                    None => Message::Positioned,
                })),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let s = self.shared.lock().unwrap();
                self.stage = s.stage.clone();
                self.transcript = s.transcript.clone();
                self.answer = s.answer.clone();
                drop(s);

                if matches!(self.stage, Stage::Done | Stage::Failed(_)) {
                    if self.done_since.is_none() {
                        self.done_since = Some(Instant::now());
                    }
                    if self.done_since.unwrap().elapsed() >= IDLE_CLOSE_AFTER_DONE {
                        return iced::exit();
                    }
                }
                Task::none()
            }
            Message::Close => iced::exit(),
            Message::Positioned => Task::none(),
            Message::Reveal(id) => window::change_mode(id, window::Mode::Windowed),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let key_events = iced::event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) =>
            {
                Some(Message::Close)
            }
            // Click-away: this is a borderless popup, so losing focus
            // (clicking the desktop, another window, the bar) should
            // dismiss it the same way Escape does — same convention as
            // og-hotkeys/og-search.
            Event::Window(iced::window::Event::Unfocused) => Some(Message::Close),
            _ => None,
        });
        let tick = iced::time::every(pipeline::POLL_INTERVAL).map(|_| Message::Tick);
        iced::Subscription::batch([key_events, tick])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let colors = AppColors::from_config(&self.config, APP_TINT_SEED);

        let status_label = match &self.stage {
            Stage::StartingStt => "Starting transcription…",
            Stage::Recording => "Listening…",
            Stage::Processing => "Processing…",
            Stage::Done => "Done",
            Stage::Failed(_) => "Error",
        };
        let status_color = match &self.stage {
            Stage::Failed(_) => Color::from_rgb(0.9, 0.35, 0.35),
            Stage::Done => colors.accent,
            _ => colors.dim_text,
        };

        let dot_color = match self.stage {
            Stage::Recording => Color::from_rgb(0.9, 0.3, 0.3),
            Stage::Processing | Stage::StartingStt => colors.accent,
            Stage::Done => Color::from_rgb(0.35, 0.8, 0.45),
            Stage::Failed(_) => Color::from_rgb(0.9, 0.35, 0.35),
        };
        let dot = container(text(""))
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(dot_color)),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });

        let header = row![
            dot,
            text("og-voice").size(14).style(move |_| text::Style { color: Some(colors.text) }),
            iced::widget::horizontal_space(),
            text(status_label).size(13).style(move |_| text::Style { color: Some(status_color) }),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(8);

        let transcript_block: Element<'_, Message> = if self.transcript.is_empty() {
            text(match self.stage {
                Stage::StartingStt => "Warming up the STT server…",
                _ => "Say something…",
            })
            .size(13)
            .style(move |_| text::Style { color: Some(colors.dim_text) })
            .into()
        } else {
            text(self.transcript.clone())
                .size(14)
                .style(move |_| text::Style { color: Some(colors.text) })
                .into()
        };

        let mut body = column![
            container(transcript_block).padding(10).width(Length::Fill).style(move |_| container::Style {
                background: Some(Background::Color(colors.surface)),
                border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                ..Default::default()
            }),
        ]
        .spacing(10);

        match &self.stage {
            Stage::Done => {
                body = body.push(
                    container(
                        text(self.answer.clone())
                            .size(14)
                            .style(move |_| text::Style { color: Some(colors.text) }),
                    )
                    .padding(10)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(colors.accent_fill),
                        border: Border { radius: colors.radius.into(), ..Default::default() },
                        ..Default::default()
                    }),
                );
            }
            Stage::Failed(err) => {
                body = body.push(
                    container(
                        text(err.clone())
                            .size(13)
                            .style(move |_| text::Style { color: Some(Color::WHITE) }),
                    )
                    .padding(10)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.6, 0.2, 0.2))),
                        border: Border { radius: colors.radius.into(), ..Default::default() },
                        ..Default::default()
                    }),
                );
            }
            _ => {}
        }

        let content = column![header, scrollable(body).height(Length::Fill)].spacing(10).padding(14);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(colors.bg_fill),
                border: Border { color: colors.accent, width: 2.0, radius: colors.radius.into() },
                ..Default::default()
            })
            .into()
    }
}
