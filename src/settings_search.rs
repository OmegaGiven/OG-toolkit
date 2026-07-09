/// Mirrors og-settings' own Tab::label() / --tab argument pairs, plus extra
/// keywords for settings that live inside a tab without being in its own
/// label — e.g. Bluetooth controls are on the Network tab, so "bluetooth"
/// needs to match there even though the tab is literally titled "Network".
/// Kept as a small static list here rather than a dependency on
/// sway-control's own crate (og-settings isn't structured as a library
/// og-search could import) — if a tab's label, --tab argument, or which
/// settings live on it changes there, it needs to change here too.
struct TabEntry {
    tab_arg: &'static str,
    label: &'static str,
    keywords: &'static [&'static str],
}

const TABS: &[TabEntry] = &[
    TabEntry { tab_arg: "power", label: "Power", keywords: &["sleep", "suspend", "lock", "startup", "services"] },
    TabEntry { tab_arg: "display", label: "Display", keywords: &["monitor", "resolution", "arrange"] },
    TabEntry { tab_arg: "network", label: "Network", keywords: &["wifi", "ethernet", "bluetooth"] },
    TabEntry { tab_arg: "updates", label: "Updates", keywords: &["pacman", "aur", "flatpak", "upgrade"] },
    TabEntry { tab_arg: "hotkeys", label: "Hotkeys", keywords: &["keybind", "shortcut", "binding"] },
    TabEntry { tab_arg: "theme", label: "Theme", keywords: &["color", "colour", "wallpaper", "cursor", "gaps", "border", "font"] },
    TabEntry { tab_arg: "bar", label: "Bar", keywords: &["waybar", "og-bar", "taskbar", "clock", "workspaces"] },
    TabEntry { tab_arg: "search", label: "Search", keywords: &["og-search", "file search"] },
    TabEntry { tab_arg: "notifications", label: "Notifications", keywords: &["mako", "dnd", "do not disturb"] },
    TabEntry { tab_arg: "history", label: "History", keywords: &[] },
    TabEntry { tab_arg: "sysmonitor", label: "System Monitor", keywords: &["cpu", "ram", "memory", "btop", "htop"] },
];

#[derive(Debug, Clone, Copy)]
pub struct SettingsMatch {
    pub tab_arg: &'static str,
    pub label: &'static str,
}

pub fn search(query: &str) -> Vec<SettingsMatch> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let q = query.to_lowercase();
    TABS.iter()
        .filter(|t| t.label.to_lowercase().contains(&q) || t.keywords.iter().any(|k| k.contains(&q as &str)))
        .map(|t| SettingsMatch { tab_arg: t.tab_arg, label: t.label })
        .collect()
}
