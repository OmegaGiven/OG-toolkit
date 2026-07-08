use iced_layershell::to_layer_message;

use crate::modules::workspaces::WorkspaceInfo;
use crate::power::PowerAction;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesUpdated(Vec<WorkspaceInfo>),
    FocusWorkspace(String),
    Tick,
    Launch(String),
    OpenPowerMenu,
    PowerAction(PowerAction),
    ClosePopup,
}
