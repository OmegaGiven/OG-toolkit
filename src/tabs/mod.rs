pub mod display;
pub mod history;
pub mod hotkeys;
pub mod network;
pub mod power;
pub mod sysmon;
pub mod taskbar_arrange;
pub mod theme;
pub mod updater;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Power,
    Display,
    Network,
    Updates,
    Hotkeys,
    Theme,
    History,
    SysMonitor,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Power,
            Tab::Display,
            Tab::Network,
            Tab::Updates,
            Tab::Hotkeys,
            Tab::Theme,
            Tab::History,
            Tab::SysMonitor,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Power => "Power",
            Tab::Display => "Display",
            Tab::Network => "Network",
            Tab::Updates => "Updates",
            Tab::Hotkeys => "Hotkeys",
            Tab::Theme => "Theme",
            Tab::History => "History",
            Tab::SysMonitor => "System Monitor",
        }
    }
}
