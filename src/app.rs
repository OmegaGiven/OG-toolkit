use iced::widget::{column, container, row, text, text_input};
use iced::{Background, Border, Color, Element, Event, Length, Task};
use iced::keyboard;

use crate::apps::{filter_apps, load_app_registry, AppEntry};
use crate::config::Config;
use crate::launch;

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    MoveSelection(i32),
    Activate,
    Close,
}

/// What pressing Enter on the currently-selected row actually does.
enum Action<'a> {
    GoAlias(String),
    WebSearch(String),
    AskAi(String),
    LaunchApp(&'a AppEntry),
}

pub struct App {
    config: Config,
    apps: Vec<AppEntry>,
    query: String,
    selected: usize,
}

fn hex_to_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return Color::BLACK;
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128);
    Color::from_rgb8(r, g, b)
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config: Config::load(),
                apps: load_app_registry(),
                query: String::new(),
                selected: 0,
            },
            text_input::focus(text_input::Id::new("query")),
        )
    }

    /// `go/<alias>` is its own distinct mode — no suggestion list, just the
    /// one action. Anything else: the single best-matching app first (if
    /// there is one), then web search, then ask AI, then the rest of the
    /// matching apps.
    fn actions(&self) -> Vec<Action<'_>> {
        let trimmed = self.query.trim();
        if let Some(alias) = trimmed.strip_prefix("go/") {
            if !alias.is_empty() {
                return vec![Action::GoAlias(alias.to_string())];
            }
        }
        let matched = filter_apps(&self.apps, &self.query);
        let mut actions = Vec::new();

        let mut rest = matched.into_iter();
        if !self.query.trim().is_empty() {
            if let Some(best) = rest.next() {
                actions.push(Action::LaunchApp(best));
            }
        }
        actions.push(Action::WebSearch(self.query.clone()));
        actions.push(Action::AskAi(self.query.clone()));
        actions.extend(rest.take(8).map(Action::LaunchApp));
        actions
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueryChanged(q) => {
                self.query = q;
                self.selected = 0;
                Task::none()
            }
            Message::MoveSelection(delta) => {
                let len = self.actions().len();
                if len > 0 {
                    let next = self.selected as i32 + delta;
                    self.selected = next.clamp(0, len as i32 - 1) as usize;
                }
                Task::none()
            }
            Message::Activate => {
                match self.actions().into_iter().nth(self.selected) {
                    Some(Action::GoAlias(alias)) => launch::go_alias(&self.config, &alias),
                    Some(Action::WebSearch(q)) if !q.trim().is_empty() => launch::web_search(&self.config, &q),
                    Some(Action::AskAi(q)) if !q.trim().is_empty() => launch::ask_ai(&self.config, &q),
                    Some(Action::LaunchApp(entry)) => launch::launch_app(&self.config, &entry.exec),
                    _ => return Task::none(),
                }
                iced::exit()
            }
            Message::Close => iced::exit(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                match key {
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Some(Message::MoveSelection(1)),
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Some(Message::MoveSelection(-1)),
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Close),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    pub fn view(&self) -> Element<'_, Message> {
        let bar_bg = crate::config::apply_color_variance(
            hex_to_color(&self.config.bar_bg), crate::config::APP_TINT_SEED,
            self.config.color_variance_enabled, self.config.color_variance_amount,
        );
        let sec_bg = crate::config::apply_color_variance(
            hex_to_color(&self.config.sec_bg), crate::config::APP_TINT_SEED,
            self.config.color_variance_enabled, self.config.color_variance_amount,
        );
        let text_color = hex_to_color(&self.config.bar_text);
        let accent = hex_to_color(&self.config.accent);
        let dim = Color { a: 0.6, ..text_color };
        let radius = self.config.corner_radius;

        let input = text_input("Search apps, or type go/<alias>…", &self.query)
            .id(text_input::Id::new("query"))
            .on_input(Message::QueryChanged)
            .on_submit(Message::Activate)
            .padding(14)
            .size(20)
            .style(move |_, _| text_input::Style {
                background: Background::Color(sec_bg),
                border: Border { color: accent, width: 2.0, radius: radius.into() },
                icon: dim,
                placeholder: dim,
                value: text_color,
                selection: accent,
            });

        let actions = self.actions();
        let rows: Vec<Element<Message>> = actions.iter().enumerate().map(|(i, action)| {
            let is_selected = i == self.selected;
            let (label, hint): (String, &str) = match action {
                Action::GoAlias(alias) => (format!("Open go/{alias}"), "internal link"),
                Action::WebSearch(q) => (format!("Web search: {q}"), "web search"),
                Action::AskAi(q) => (
                    format!("Ask {}: {q}", if self.config.default_ai_cli.is_empty() { "AI" } else { &self.config.default_ai_cli }),
                    "ai",
                ),
                Action::LaunchApp(entry) => (entry.name.clone(), "app"),
            };

            container(
                row![
                    text(label).size(16).style(move |_| text::Style {
                        color: Some(if is_selected { bar_bg } else { text_color }),
                    }),
                    iced::widget::horizontal_space(),
                    text(hint).size(12).style(move |_| text::Style {
                        color: Some(if is_selected { bar_bg } else { dim }),
                    }),
                ]
                .align_y(iced::Alignment::Center)
            )
            .padding([10, 14])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(if is_selected { accent } else { Color::TRANSPARENT })),
                border: Border { radius: radius.into(), ..Default::default() },
                ..Default::default()
            })
            .into()
        }).collect();

        let list = column(rows).spacing(2);

        container(
            column![input, list]
                .spacing(10)
                .padding(16)
        )
        .width(640)
        .style(move |_| container::Style {
            background: Some(Background::Color(bar_bg)),
            border: Border { color: accent, width: 2.0, radius: radius.into() },
            ..Default::default()
        })
        .into()
    }
}
