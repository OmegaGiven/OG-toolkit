//! Workspace indicators. Shells out to `swaymsg` rather than speaking the
//! sway IPC socket protocol directly — every other OG-toolkit/sway-config
//! integration already goes through `swaymsg`, so this keeps og-bar
//! consistent instead of adding a bespoke IPC client for one module.

use iced::widget::{button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Subscription};
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use og_config::IconRewriteRule;

use crate::icon_font;
use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

#[derive(Debug, Clone, Deserialize)]
struct RawWorkspace {
    num: i32,
    name: String,
    focused: bool,
}

/// One window's resolved icon, paired with its con_id so clicking that
/// specific icon can focus that specific window — not just switch to
/// whichever workspace it happens to be on.
#[derive(Debug, Clone)]
pub struct WindowIcon {
    pub con_id: i64,
    pub icon: String,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub num: i32,
    pub name: String,
    pub focused: bool,
    /// Already-resolved icon per window on this workspace (rewrite applied
    /// at fetch time so `view()` doesn't need the rule table at all).
    pub windows: Vec<WindowIcon>,
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
        // U+F128 (nf-fa-question) rather than ASCII "?" — Symbols Nerd Font
        // has zero ASCII coverage, so a plain "?" here would tofu right
        // alongside the icons it's meant to sit next to.
        .unwrap_or_else(|| "\u{f128}".to_string())
}

/// Walks sway's `get_tree` output collecting, per top-level workspace node,
/// the resolved icon (and con_id, for per-window focus-on-click) for every
/// window nested anywhere under it (splits/tabs nest windows arbitrarily
/// deep, so this recurses rather than assuming a flat child list).
fn collect_windows(tree: &serde_json::Value, rules: &[IconRewriteRule]) -> std::collections::HashMap<String, Vec<WindowIcon>> {
    let mut out = std::collections::HashMap::new();

    fn walk(node: &serde_json::Value, current_ws: Option<&str>, rules: &[IconRewriteRule], out: &mut std::collections::HashMap<String, Vec<WindowIcon>>) {
        let node_type = node.get("type").and_then(|v| v.as_str());
        let ws_name = if node_type == Some("workspace") {
            node.get("name").and_then(|v| v.as_str())
        } else {
            current_ws
        };

        let app_id = node.get("app_id").and_then(|v| v.as_str());
        let class = node.get("window_properties").and_then(|wp| wp.get("class")).and_then(|v| v.as_str());
        if let (Some(con_id), Some(ws_name)) = (node.get("id").and_then(|v| v.as_i64()), ws_name) {
            if app_id.is_some() || class.is_some() {
                let focused = node.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
                out.entry(ws_name.to_string())
                    .or_default()
                    .push(WindowIcon { con_id, icon: resolve_icon(app_id, class, rules), focused });
            }
        }

        for key in ["nodes", "floating_nodes"] {
            if let Some(children) = node.get(key).and_then(|v| v.as_array()) {
                for child in children {
                    walk(child, ws_name, rules, out);
                }
            }
        }
    }

    walk(tree, None, rules, &mut out);
    out
}

async fn fetch_workspaces(rules: &[IconRewriteRule]) -> Vec<WorkspaceInfo> {
    let workspaces_out = Command::new("swaymsg").args(["-t", "get_workspaces"]).output().await;
    let raw: Vec<RawWorkspace> = match workspaces_out {
        Ok(out) => serde_json::from_slice(&out.stdout).unwrap_or_default(),
        Err(_) => return Vec::new(),
    };

    let tree_out = Command::new("swaymsg").args(["-t", "get_tree"]).output().await;
    let windows_by_ws = match tree_out {
        Ok(out) => serde_json::from_slice::<serde_json::Value>(&out.stdout)
            .map(|tree| collect_windows(&tree, rules))
            .unwrap_or_default(),
        Err(_) => std::collections::HashMap::new(),
    };

    raw.into_iter()
        .map(|w| WorkspaceInfo {
            windows: windows_by_ws.get(&w.name).cloned().unwrap_or_default(),
            num: w.num,
            name: w.name,
            focused: w.focused,
        })
        .collect()
}

pub struct Workspaces {
    workspaces: Vec<WorkspaceInfo>,
    icon_rewrite: Vec<IconRewriteRule>,
}

impl Workspaces {
    pub fn new(icon_rewrite: Vec<IconRewriteRule>) -> Self {
        Self { workspaces: Vec::new(), icon_rewrite }
    }
}

impl Module for Workspaces {
    fn cell_length(&self, _size: u32, _orientation: Orientation) -> Length {
        Length::Shrink
    }

    fn view(&self, colors: AppColors, size: u32, orientation: Orientation) -> Element<'_, Message> {
        let groups: Vec<Element<Message>> = self
            .workspaces
            .iter()
            .map(|ws| {
                let bg = if ws.focused { colors.accent } else { Color::TRANSPARENT };
                let fg = if ws.focused { colors.bar_bg } else { colors.text };

                let number_btn = button(
                    container(text(ws.name.clone()).size(14).style(move |_| text::Style { color: Some(fg) }))
                        .width(size as u16)
                        .height(size as u16)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                )
                .padding(0)
                .style(move |_, _| button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { radius: colors.radius.into(), ..Default::default() },
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Message::FocusWorkspace(ws.name.clone()));

                // Each window is its own clickable button (FocusWindow,
                // not FocusWorkspace) — clicking a specific app's icon
                // jumps straight to that window, not just its workspace.
                // Icon-rewrite entries are user-editable and not all
                // nerd-font glyphs (e.g. "alacritty" maps to plain "./"),
                // so font_for() picks per-icon rather than forcing one.
                let window_btns = ws.windows.iter().map(|w| {
                    let con_id = w.con_id;
                    let win_focused = w.focused;
                    let win_fg = if win_focused { colors.bar_bg } else { colors.text };
                    button(
                        container(
                            text(w.icon.clone())
                                .size(14)
                                .font(icon_font::font_for(&w.icon))
                                .style(move |_| text::Style { color: Some(win_fg) }),
                        )
                        .width(size as u16)
                        .height(size as u16)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                    )
                    .padding(0)
                    .style(move |_, status| button::Style {
                        background: Some(Background::Color(if win_focused {
                            colors.accent
                        } else if matches!(status, button::Status::Hovered) {
                            colors.header_btn_bg
                        } else {
                            Color::TRANSPARENT
                        })),
                        border: Border { radius: colors.radius.into(), ..Default::default() },
                        text_color: win_fg,
                        ..Default::default()
                    })
                    .on_press(Message::FocusWindow(con_id))
                    .into()
                });

                let mut parts: Vec<Element<Message>> = vec![number_btn.into()];
                parts.extend(window_btns);

                let group: Element<Message> = match orientation {
                    Orientation::Horizontal => row(parts).spacing(1).into(),
                    Orientation::Vertical => column(parts).spacing(1).into(),
                };

                // Outline around the number + its app icons together, so
                // it's visually obvious which icons belong to which
                // desktop — every workspace gets one (not just the
                // focused one), so they read as separate groups at a
                // glance rather than one continuous strip of icons.
                container(group)
                    .padding(2)
                    .style(move |_| container::Style {
                        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                        ..Default::default()
                    })
                    .into()
            })
            .collect();

        match orientation {
            Orientation::Horizontal => row(groups).spacing(4).into(),
            Orientation::Vertical => column(groups).spacing(4).into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let rules = self.icon_rewrite.clone();
        Subscription::run_with_id("workspaces", workspace_stream(rules))
    }

    fn update(&mut self, message: &Message) {
        if let Message::WorkspacesUpdated(list) = message {
            let mut list = list.clone();
            list.sort_by_key(|ws| ws.num);
            self.workspaces = list;
        }
    }
}

fn workspace_stream(rules: Vec<IconRewriteRule>) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, |mut sender| async move {
        use iced::futures::SinkExt;

        let _ = sender.send(Message::WorkspacesUpdated(fetch_workspaces(&rules).await)).await;

        // kill_on_drop only covers a graceful stream teardown; if og-bar
        // itself is killed (SIGTERM/SIGKILL) the drop glue never runs, so
        // PR_SET_PDEATHSIG is what actually prevents an orphaned `swaymsg
        // subscribe` surviving a crashed/killed bar across dev restarts.
        let mut command = Command::new("swaymsg");
        command
            // Window rewrite icons need to refresh on window open/close/
            // move too, not just workspace focus changes — waybar's own
            // module implicitly does the same since it re-derives from
            // the same tree on every relevant event.
            .args(["-t", "subscribe", "-m", r#"["workspace","window"]"#])
            .stdout(Stdio::piped())
            .kill_on_drop(true);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
        let Ok(mut child) = command.spawn()
        else {
            return;
        };
        let Some(stdout) = child.stdout.take() else { return };
        let mut lines = BufReader::new(stdout).lines();

        while let Ok(Some(_line)) = lines.next_line().await {
            let _ = sender.send(Message::WorkspacesUpdated(fetch_workspaces(&rules).await)).await;
        }
    })
}
