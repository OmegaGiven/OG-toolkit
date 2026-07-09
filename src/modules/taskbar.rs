//! Flat list of every open window, not grouped by workspace like
//! Workspaces is — click one to jump straight to it (`swaymsg '[con_id=X]
//! focus'`) regardless of which workspace it's actually on. Shares the
//! same icon_rewrite resolution as Workspaces' per-window icons.

use iced::widget::{button, column, row, text};
use iced::{Background, Border, Color, Element, Length, Subscription};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use og_config::IconRewriteRule;

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

#[derive(Debug, Clone)]
pub struct TaskbarWindow {
    pub con_id: i64,
    pub icon: String,
    pub title: String,
}

fn resolve_icon(app_id: Option<&str>, class: Option<&str>, rules: &[IconRewriteRule]) -> String {
    let haystack = format!(
        "{} {}",
        app_id.unwrap_or_default().to_lowercase(),
        class.unwrap_or_default().to_lowercase()
    );
    rules
        .iter()
        .find(|r| haystack.contains(&r.match_value.to_lowercase()))
        .map(|r| r.icon.clone())
        .unwrap_or_else(|| "\u{f128}".to_string())
}

fn collect_windows(tree: &serde_json::Value, rules: &[IconRewriteRule], out: &mut Vec<TaskbarWindow>) {
    let app_id = tree.get("app_id").and_then(|v| v.as_str());
    let class = tree.get("window_properties").and_then(|wp| wp.get("class")).and_then(|v| v.as_str());
    if let (Some(con_id), true) = (tree.get("id").and_then(|v| v.as_i64()), app_id.is_some() || class.is_some()) {
        out.push(TaskbarWindow {
            con_id,
            icon: resolve_icon(app_id, class, rules),
            title: tree.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        });
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = tree.get(key).and_then(|v| v.as_array()) {
            for child in children {
                collect_windows(child, rules, out);
            }
        }
    }
}

async fn fetch_windows(rules: &[IconRewriteRule]) -> Vec<TaskbarWindow> {
    let out = Command::new("swaymsg").args(["-t", "get_tree"]).output().await;
    match out {
        Ok(out) => {
            let mut windows = Vec::new();
            if let Ok(tree) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                collect_windows(&tree, rules, &mut windows);
            }
            windows
        }
        Err(_) => Vec::new(),
    }
}

pub struct Taskbar {
    windows: Vec<TaskbarWindow>,
    icon_rewrite: Vec<IconRewriteRule>,
}

impl Taskbar {
    pub fn new(icon_rewrite: Vec<IconRewriteRule>) -> Self {
        Self { windows: Vec::new(), icon_rewrite }
    }
}

impl Module for Taskbar {
    fn cell_length(&self, _size: u32, _orientation: Orientation) -> Length {
        Length::Shrink
    }

    fn view(&self, colors: AppColors, size: u32, orientation: Orientation) -> Element<'_, Message> {
        let buttons: Vec<Element<Message>> = self
            .windows
            .iter()
            .map(|w| {
                let fg = colors.text;
                let btn = button(
                    text(w.icon.clone())
                        .size(14)
                        .font(icon_font::font_for(&w.icon))
                        .style(move |_| text::Style { color: Some(fg) }),
                )
                .padding(4)
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
                .on_press(Message::FocusWindow(w.con_id));

                // Multiple windows often share the same icon (two
                // terminals, two browser windows) — the title is the only
                // thing that actually distinguishes them.
                iced::widget::tooltip(
                    btn,
                    iced::widget::container(text(w.title.clone()).size(12).style(move |_| text::Style { color: Some(colors.text) }))
                        .padding(6)
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Background::Color(colors.surface)),
                            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                            ..Default::default()
                        }),
                    iced::widget::tooltip::Position::Bottom,
                )
                .into()
            })
            .collect();
        let _ = size;

        match orientation {
            Orientation::Horizontal => row(buttons).spacing(2).into(),
            Orientation::Vertical => column(buttons).spacing(2).into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let rules = self.icon_rewrite.clone();
        Subscription::run_with_id("taskbar", taskbar_stream(rules))
    }

    fn update(&mut self, message: &Message) {
        if let Message::TaskbarUpdated(list) = message {
            self.windows = list.clone();
        }
    }
}

fn taskbar_stream(rules: Vec<IconRewriteRule>) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, |mut sender| async move {
        use iced::futures::SinkExt;

        let _ = sender.send(Message::TaskbarUpdated(fetch_windows(&rules).await)).await;

        let mut command = Command::new("swaymsg");
        command
            .args(["-t", "subscribe", "-m", r#"["window"]"#])
            .stdout(Stdio::piped())
            .kill_on_drop(true);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
        let Ok(mut child) = command.spawn() else { return };
        let Some(stdout) = child.stdout.take() else { return };
        let mut lines = BufReader::new(stdout).lines();

        while let Ok(Some(_line)) = lines.next_line().await {
            let _ = sender.send(Message::TaskbarUpdated(fetch_windows(&rules).await)).await;
        }
    })
}
