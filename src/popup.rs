//! One reusable anchored-popup mechanism (PLAN.md section 4). A popup is
//! just a second layer-shell surface, positioned by the compositor at the
//! point the user clicked (via `NewMenu`'s direction). Every menu the bar
//! ever needs — power, network, bluetooth, tray items, the customization
//! popout — is a consumer of this one `PopupState`, not its own surface
//! plumbing.

use iced::window;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    Power,
}

#[derive(Debug, Clone)]
pub struct PopupState {
    pub id: window::Id,
    pub kind: PopupKind,
}
