//! Workspace indicators. Shells out to `swaymsg` rather than speaking the
//! sway IPC socket protocol directly — every other OG-toolkit/sway-config
//! integration already goes through `swaymsg`, so this keeps og-bar
//! consistent instead of adding a bespoke IPC client for one module.

use iced::widget::{button, container, row, text};
use iced::{Background, Border, Color, Element, Length, Subscription};
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::message::Message;
use crate::module::Module;
use og_theme::AppColors;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceInfo {
    pub num: i32,
    pub name: String,
    pub focused: bool,
}

async fn fetch_workspaces() -> Vec<WorkspaceInfo> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .output()
        .await;
    match output {
        Ok(out) => serde_json::from_slice(&out.stdout).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub struct Workspaces {
    workspaces: Vec<WorkspaceInfo>,
}

impl Workspaces {
    pub fn new() -> Self {
        Self { workspaces: Vec::new() }
    }
}

impl Module for Workspaces {
    fn cell_width(&self, _size: u32) -> Length {
        Length::Shrink
    }

    fn view(&self, colors: AppColors, size: u32) -> Element<'_, Message> {
        let buttons: Vec<Element<Message>> = self
            .workspaces
            .iter()
            .map(|ws| {
                let bg = if ws.focused { colors.accent } else { Color::TRANSPARENT };
                let fg = if ws.focused { colors.bar_bg } else { colors.text };
                button(
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
                .on_press(Message::FocusWorkspace(ws.name.clone()))
                .into()
            })
            .collect();

        row(buttons).spacing(2).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(workspace_stream)
    }

    fn update(&mut self, message: &Message) {
        if let Message::WorkspacesUpdated(list) = message {
            let mut list = list.clone();
            list.sort_by_key(|ws| ws.num);
            self.workspaces = list;
        }
    }
}

fn workspace_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, |mut sender| async move {
        use iced::futures::SinkExt;

        let _ = sender.send(Message::WorkspacesUpdated(fetch_workspaces().await)).await;

        // kill_on_drop only covers a graceful stream teardown; if og-bar
        // itself is killed (SIGTERM/SIGKILL) the drop glue never runs, so
        // PR_SET_PDEATHSIG is what actually prevents an orphaned `swaymsg
        // subscribe` surviving a crashed/killed bar across dev restarts.
        let mut command = Command::new("swaymsg");
        command
            .args(["-t", "subscribe", "-m", r#"["workspace"]"#])
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
            let _ = sender.send(Message::WorkspacesUpdated(fetch_workspaces().await)).await;
        }
    })
}
