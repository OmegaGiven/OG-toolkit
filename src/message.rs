use iced_layershell::to_layer_message;
use og_config::{Edge, ModuleKind};

use crate::modules::tray::TrayItem;
use crate::modules::workspaces::WorkspaceInfo;
use crate::power::PowerAction;
use crate::settings::Section;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesUpdated(Vec<WorkspaceInfo>),
    FocusWorkspace(String),
    FocusWindow(i64),
    TrayUpdated(Vec<TrayItem>),
    TrayActivate(String),
    Tick,
    Launch(String),
    PulseaudioToggleMute,
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
    RemoveModule(Section, usize),
    AddModule(Section, ModuleKind),
    SetClockTimezone(Section, usize, String),
    ApplyRelayout,
}
