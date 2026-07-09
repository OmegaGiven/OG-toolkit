use iced_layershell::to_layer_message;
use og_config::Edge;

use crate::modules::workspaces::WorkspaceInfo;
use crate::power::PowerAction;
use crate::settings::Section;

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
    ClickOnWindow(iced::window::Id),
    OpenSettingsPopup,
    SetEdge(Edge),
    SetThickness(u32),
    SetItemSize(u32),
    SetSpacing(u32),
    SetPadding(u32),
    ToggleModule(Section, usize),
    SetClockTimezone(Section, usize, String),
    ApplyRelayout,
}
