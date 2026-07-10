use iced_layershell::to_layer_message;
use og_config::{Edge, ModuleKind, SectionAlign};

use crate::modules::tray::TrayItem;
use crate::modules::workspaces::WorkspaceInfo;
use crate::power::PowerAction;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesUpdated(Vec<WorkspaceInfo>),
    FocusWorkspace(String),
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
    // section index (position in bar_cfg.sections at render time, not
    // BarSection::id — messages round-trip within a single synchronous
    // update() from a just-rendered view, so the list can't shift under
    // them in a way an index would get wrong).
    ToggleModule(usize, usize),
    RemoveModule(usize, usize),
    AddModule(usize, ModuleKind),
    SetClockTimezone(usize, usize, String),
    AddSection,
    RemoveSection(usize),
    SetSectionPercent(usize, u32),
    SetSectionAlign(usize, SectionAlign),
    ApplyRelayout,
    OpenWindowMenu(i64, i32),
    WindowMenuClose(i64),
    WindowMenuMoveToWorkspace(i64, i32),
    // Click-hold-drag an app icon onto another workspace's group to move
    // it there. Start carries the window's *current* workspace so the
    // eventual drop can tell "still over where it started" (no-op) apart
    // from "actually moved somewhere else."
    WindowDragStart(i64, i32),
    WorkspaceGroupHovered(i32),
    WindowDragEnd,
}
