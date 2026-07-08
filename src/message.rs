use iced_layershell::to_layer_message;

use crate::modules::workspaces::WorkspaceInfo;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesUpdated(Vec<WorkspaceInfo>),
    FocusWorkspace(String),
    Tick,
    Launch(String),
}
