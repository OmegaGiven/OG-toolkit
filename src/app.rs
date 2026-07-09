use iced::widget::{column, container, row, text, text_input};
use iced::{Background, Border, Color, Element, Event, Length, Task};
use iced::keyboard;

use std::path::PathBuf;

use crate::apps::{filter_apps, load_app_registry, AppEntry};
use crate::config::{Config, APP_TINT_SEED};
use crate::file_search;
use crate::launch;
use crate::settings_search::{self, SettingsMatch};
use og_theme::{apply_color_variance, hex_to_color};

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    MoveSelection(i32),
    Activate,
    Close,
    /// Carries the query it was searched *for* — file search is async, so
    /// a slow result for an old keystroke arriving after the user has kept
    /// typing must be discarded, not rendered against the wrong query.
    FileResults(String, Vec<PathBuf>),
}

/// Window is a fixed 420px tall with no scrollable — this is how many rows
/// fit before the list would start rendering off the bottom edge.
const VISIBLE_ROWS: usize = 7;

/// What pressing Enter on the currently-selected row actually does.
enum Action<'a> {
    GoAlias(String),
    WebSearch(String),
    AskAi(String),
    LaunchApp(&'a AppEntry),
    OpenFile(&'a PathBuf),
    OpenSetting(SettingsMatch),
}

pub struct App {
    config: Config,
    apps: Vec<AppEntry>,
    query: String,
    selected: usize,
    /// Only rendered when `file_results_query == query` — guards against
    /// showing results for whatever the user was typing a few keystrokes
    /// ago while this search was still in flight.
    file_results: Vec<PathBuf>,
    file_results_query: String,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config: Config::load(),
                apps: load_app_registry(),
                query: String::new(),
                selected: 0,
                file_results: Vec::new(),
                file_results_query: String::new(),
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

        // Both last, in that order — real filesystem I/O and settings
        // lookups are lower-priority than anything above, which is either
        // free (web/AI) or already-known-installed apps.
        if self.config.search_files_enabled && self.file_results_query == self.query {
            actions.extend(self.file_results.iter().map(Action::OpenFile));
        }
        if self.config.search_settings_enabled {
            actions.extend(settings_search::search(&self.query).into_iter().map(Action::OpenSetting));
        }

        actions
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueryChanged(q) => {
                self.query = q.clone();
                self.selected = 0;
                if self.config.search_files_enabled && !q.trim().is_empty() {
                    Task::future(async move { Message::FileResults(q.clone(), file_search::search(q).await) })
                } else {
                    Task::none()
                }
            }
            Message::FileResults(for_query, results) => {
                if for_query == self.query {
                    self.file_results_query = for_query;
                    self.file_results = results;
                }
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
                    Some(Action::OpenFile(path)) => launch::open_file(path),
                    Some(Action::OpenSetting(m)) => launch::open_settings_tab(m.tab_arg),
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
        let bar_bg = apply_color_variance(
            hex_to_color(&self.config.bar_bg), APP_TINT_SEED,
            self.config.color_variance_enabled, self.config.color_variance_amount,
        );
        let sec_bg = apply_color_variance(
            hex_to_color(&self.config.sec_bg), APP_TINT_SEED,
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

        // The list isn't in a scrollable — it's a fixed-height window — so
        // without this, selecting past the last visible row just renders
        // off the bottom edge instead of scrolling. Show a sliding window
        // of rows around the selection instead, shifting up once the
        // selection would fall past the last visible slot.
        let scroll_offset = self.selected.saturating_sub(VISIBLE_ROWS.saturating_sub(1));
        let visible = actions.iter().enumerate().skip(scroll_offset).take(VISIBLE_ROWS);

        let rows: Vec<Element<Message>> = visible.map(|(i, action)| {
            let is_selected = i == self.selected;
            let (label, hint): (String, &str) = match action {
                Action::GoAlias(alias) => (format!("Open go/{alias}"), "internal link"),
                Action::WebSearch(q) => (format!("Web search: {q}"), "web search"),
                Action::AskAi(q) => (
                    format!("Ask {}: {q}", if self.config.default_ai_cli.is_empty() { "AI" } else { &self.config.default_ai_cli }),
                    "ai",
                ),
                Action::LaunchApp(entry) => (entry.name.clone(), "app"),
                Action::OpenFile(path) => (path.display().to_string(), "file"),
                Action::OpenSetting(m) => (format!("Settings: {}", m.label), "settings"),
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
