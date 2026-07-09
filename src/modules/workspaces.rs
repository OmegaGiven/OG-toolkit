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

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

#[derive(Debug, Clone, Deserialize)]
struct RawWorkspace {
    num: i32,
    name: String,
    focused: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub num: i32,
    pub name: String,
    pub focused: bool,
    /// Already-resolved icon per window on this workspace (rewrite applied
    /// at fetch time so `view()` doesn't need the rule table at all).
    pub window_icons: Vec<String>,
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
        .unwrap_or_else(|| "?".to_string())
}

/// Walks sway's `get_tree` output collecting, per top-level workspace node,
/// the resolved icon for every window nested anywhere under it (splits/tabs
/// nest windows arbitrarily deep, so this recurses rather than assuming a
/// flat child list).
fn collect_window_icons(tree: &serde_json::Value, rules: &[IconRewriteRule]) -> std::collections::HashMap<String, Vec<String>> {
    let mut out = std::collections::HashMap::new();

    fn walk(node: &serde_json::Value, current_ws: Option<&str>, rules: &[IconRewriteRule], out: &mut std::collections::HashMap<String, Vec<String>>) {
        let node_type = node.get("type").and_then(|v| v.as_str());
        let ws_name = if node_type == Some("workspace") {
            node.get("name").and_then(|v| v.as_str())
        } else {
            current_ws
        };

        let app_id = node.get("app_id").and_then(|v| v.as_str());
        let class = node.get("window_properties").and_then(|wp| wp.get("class")).and_then(|v| v.as_str());
        if (app_id.is_some() || class.is_some()) && ws_name.is_some() {
            out.entry(ws_name.unwrap().to_string())
                .or_default()
                .push(resolve_icon(app_id, class, rules));
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
            .map(|tree| collect_window_icons(&tree, rules))
            .unwrap_or_default(),
        Err(_) => std::collections::HashMap::new(),
    };

    raw.into_iter()
        .map(|w| WorkspaceInfo {
            window_icons: windows_by_ws.get(&w.name).cloned().unwrap_or_default(),
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
        let buttons: Vec<Element<Message>> = self
            .workspaces
            .iter()
            .map(|ws| {
                let bg = if ws.focused { colors.accent } else { Color::TRANSPARENT };
                let fg = if ws.focused { colors.bar_bg } else { colors.text };
                let label = if ws.window_icons.is_empty() {
                    ws.name.clone()
                } else {
                    ws.window_icons.join(" ")
                };
                button(
                    container(text(label).size(14).style(move |_| text::Style { color: Some(fg) }))
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
                .on_press(Message::FocusWorkspace(ws.name.clone()))
                .into()
            })
            .collect();

        match orientation {
            Orientation::Horizontal => row(buttons).spacing(2).into(),
            Orientation::Vertical => column(buttons).spacing(2).into(),
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
