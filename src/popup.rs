//! One reusable anchored-popup mechanism (PLAN.md section 4). A popup is
//! just a second layer-shell surface, positioned by the compositor at the
//! point the user clicked (via `NewMenu`'s direction). Every menu the bar
//! ever needs — power, network, bluetooth, tray items, the customization
//! popout — is a consumer of this one `PopupState`, not its own surface
//! plumbing.

use iced::window;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupKind {
    Power,
    Settings,
    /// Right-click menu on a single app icon in the Workspaces module —
    /// close it, or move it to another workspace. `other_workspaces` is
    /// resolved once at open time (a quick synchronous `get_workspaces`,
    /// same convention as other sync shell-outs in this codebase) rather
    /// than kept live, since the menu itself is transient.
    WindowMenu {
        con_id: i64,
        other_workspaces: Vec<i32>,
    },
}

#[derive(Debug, Clone)]
pub struct PopupState {
    pub id: window::Id,
    pub kind: PopupKind,
}
