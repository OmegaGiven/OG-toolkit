//! Shared config: the single `Config` struct definition and its plain
//! load/save, used by every OG-toolkit app. Only `og-settings` (the theme
//! editor) writes this file — everyone else just reads it.
//!
//! Deliberately does *not* include any sway/waybar-config-scraping "seed
//! from live system state" logic (cursor theme, wallpaper, module order,
//! etc.) — that's only ever needed by the one app that owns first-run
//! setup, and lives there instead of here.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn config_path() -> PathBuf {
    let mut p = dirs_home();
    p.push(".config/sway-power/config.json");
    p
}

pub fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SleepSetting {
    pub enabled: bool,
    pub minutes: u64,
}

impl Default for SleepSetting {
    fn default() -> Self {
        Self { enabled: false, minutes: 15 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MonitorConfig {
    pub name: String,
    pub resolution: String,
    pub x: i32,
    pub y: i32,
    pub enabled: bool,
}

/// An additional clock widget with its own timezone. `id` is a stable,
/// never-reused counter (not an array index) so removing one clock can't
/// shift another's `clock#N` module id out from under it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClockConfig {
    pub id: u32,
    pub timezone: String,
}

impl ClockConfig {
    pub fn module_id(&self) -> String {
        format!("clock#{}", self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub monitor_sleep: SleepSetting,
    #[serde(default)]
    pub system_sleep: SleepSetting,
    /// Locks via swaylock after this many idle minutes, and always before
    /// system sleep once enabled. Read by `og-power-apply`.
    #[serde(default)]
    pub screen_lock: SleepSetting,
    #[serde(default)]
    pub gaps_inner: i32,
    #[serde(default)]
    pub gaps_outer: i32,
    #[serde(default = "default_border")]
    pub border_width: i32,
    #[serde(default = "default_bar_bg")]
    pub bar_bg: String,
    #[serde(default = "default_bar_text")]
    pub bar_text: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    /// Unfocused workspace numbers/icons in waybar, and og-settings' own
    /// box borders. Distinct from `inactive_color` (unfocused window
    /// borders) and `accent` (the primary/focused accent).
    #[serde(default = "default_accent2")]
    pub accent2: String,
    #[serde(default = "default_sec_bg")]
    pub sec_bg: String,
    #[serde(default = "default_inactive")]
    pub inactive_color: String,
    #[serde(default = "default_urgent")]
    pub urgent_color: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    /// Used by og-search for web search / `go/<alias>` lookups.
    #[serde(default)]
    pub default_browser: String,
    /// Used by og-search's "Ask AI" suggestion.
    #[serde(default)]
    pub default_ai_cli: String,
    #[serde(default)]
    pub monitor_configs: Vec<MonitorConfig>,
    #[serde(default = "default_waybar_position")]
    pub waybar_position: String,
    #[serde(default = "default_waybar_thickness")]
    pub waybar_thickness: i32,
    #[serde(default)]
    pub clock_timezone: String,
    #[serde(default = "default_clock_12h")]
    pub clock_12h: bool,
    /// Shared corner-rounding radius (pixels) for every OG-toolkit app's UI
    /// chrome.
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,
    /// When enabled, each app that shares this theme tints its background
    /// slightly differently (a small deterministic hash-based shift, not
    /// random) so overlapping windows are easier to tell apart at a glance
    /// while still visibly sharing the same base theme.
    #[serde(default)]
    pub color_variance_enabled: bool,
    /// Max shift fraction applied per-app when variance is enabled — 0.0
    /// disables it in practice, larger values make the tint more obvious.
    #[serde(default = "default_color_variance_amount")]
    pub color_variance_amount: f32,
    /// Renders background (bg → sec_bg) and accent (accent → accent2) as
    /// gradients instead of flat colors, wherever that's wired up (waybar,
    /// wofi, and og-settings' own window bg / Apply & Save button).
    #[serde(default)]
    pub gradient_enabled: bool,
    #[serde(default)]
    pub extra_clocks: Vec<ClockConfig>,
    #[serde(default)]
    pub next_clock_id: u32,
    /// Module ids per bar section (waybar's `modules-left/center/right`).
    /// Empty until first seeded from the live waybar config.
    #[serde(default)]
    pub modules_left: Vec<String>,
    #[serde(default)]
    pub modules_center: Vec<String>,
    #[serde(default)]
    pub modules_right: Vec<String>,
    /// -1.0 (slowest) .. 1.0 (fastest), sway's own `pointer_accel` range.
    #[serde(default)]
    pub mouse_sensitivity: f32,
    /// Empty until first seeded from the live sway config's `seat seat0
    /// xcursor_theme` line, alongside `cursor_size`.
    #[serde(default)]
    pub cursor_theme: String,
    #[serde(default = "default_cursor_size")]
    pub cursor_size: i32,
    /// "image" or "color".
    #[serde(default = "default_wallpaper_mode")]
    pub wallpaper_mode: String,
    /// Empty until first seeded from the live sway config's `output * bg`
    /// line, alongside `wallpaper_mode`/`cursor_theme`.
    #[serde(default)]
    pub wallpaper_path: String,
    #[serde(default = "default_wallpaper_color")]
    pub wallpaper_color: String,
    /// sway's `output bg` scaling mode: "fill", "stretch", or "center"
    /// (center = native resolution, unscaled).
    #[serde(default = "default_wallpaper_fit")]
    pub wallpaper_fit: String,
    /// Read directly by the standalone `og-notify` daemon, not og-settings
    /// — flashes a rain/glow/wind/sparkle overlay on every `Notify` dbus
    /// call.
    #[serde(default)]
    pub notif_fx_enabled: bool,
    /// "rain", "glow", "wind", or "sparkle" — mutually exclusive, not
    /// independent toggles.
    #[serde(default = "default_notif_fx_effect")]
    pub notif_fx_effect: String,
    #[serde(default = "default_notif_fx_color")]
    pub notif_fx_color: String,
    #[serde(default = "default_notif_fx_duration")]
    pub notif_fx_duration_ms: u32,
    /// 1.0 = fully opaque (effectively off). Applied to every window that
    /// isn't currently focused, by the standalone `og-opacity-apply`
    /// daemon watching sway's window-focus events.
    #[serde(default = "default_unfocused_opacity")]
    pub unfocused_opacity: f32,
}

fn default_border() -> i32 { 1 }
fn default_bar_bg() -> String { "#1a1a2e".into() }
fn default_bar_text() -> String { "#e0e0e0".into() }
fn default_accent() -> String { "#ff7800".into() }
fn default_accent2() -> String { "#8855ff".into() }
fn default_sec_bg() -> String { "#2a2535".into() }
fn default_inactive() -> String { "#3a3a4a".into() }
fn default_urgent() -> String { "#ff4444".into() }
fn default_terminal() -> String { "alacritty".into() }
fn default_waybar_position() -> String { "left".into() }
fn default_waybar_thickness() -> i32 { 32 }
fn default_clock_12h() -> bool { false }
fn default_corner_radius() -> f32 { 0.0 }
fn default_color_variance_amount() -> f32 { 0.06 }
fn default_cursor_size() -> i32 { 24 }
fn default_wallpaper_mode() -> String { "image".into() }
fn default_wallpaper_color() -> String { "#1a1a2e".into() }
fn default_wallpaper_fit() -> String { "fill".into() }
fn default_notif_fx_effect() -> String { "rain".into() }
fn default_notif_fx_color() -> String { "#ff7800".into() }
fn default_notif_fx_duration() -> u32 { 1600 }
fn default_unfocused_opacity() -> f32 { 1.0 }

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_sleep: SleepSetting { enabled: true, minutes: 15 },
            system_sleep: SleepSetting { enabled: false, minutes: 30 },
            screen_lock: SleepSetting { enabled: false, minutes: 10 },
            gaps_inner: 0,
            gaps_outer: 0,
            border_width: 1,
            bar_bg: default_bar_bg(),
            bar_text: default_bar_text(),
            accent: default_accent(),
            accent2: default_accent2(),
            sec_bg: default_sec_bg(),
            inactive_color: default_inactive(),
            urgent_color: default_urgent(),
            terminal: default_terminal(),
            default_browser: String::new(),
            default_ai_cli: String::new(),
            monitor_configs: Vec::new(),
            waybar_position: default_waybar_position(),
            waybar_thickness: default_waybar_thickness(),
            clock_timezone: String::new(),
            clock_12h: default_clock_12h(),
            corner_radius: default_corner_radius(),
            color_variance_enabled: false,
            gradient_enabled: false,
            color_variance_amount: default_color_variance_amount(),
            extra_clocks: Vec::new(),
            next_clock_id: 0,
            modules_left: Vec::new(),
            modules_center: Vec::new(),
            modules_right: Vec::new(),
            mouse_sensitivity: 0.0,
            cursor_theme: String::new(),
            cursor_size: default_cursor_size(),
            wallpaper_mode: default_wallpaper_mode(),
            wallpaper_path: String::new(),
            wallpaper_color: default_wallpaper_color(),
            wallpaper_fit: default_wallpaper_fit(),
            notif_fx_enabled: false,
            notif_fx_effect: default_notif_fx_effect(),
            notif_fx_color: default_notif_fx_color(),
            notif_fx_duration_ms: default_notif_fx_duration(),
            unfocused_opacity: default_unfocused_opacity(),
        }
    }
}

impl Config {
    /// Plain load: read the shared JSON file, defaulting any missing
    /// field. Does *not* seed anything from live sway/waybar config files
    /// — see `og-settings`'s own `config.rs` for that (first-run seeding
    /// is that app's job alone).
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}

fn bar_config_path() -> PathBuf {
    let mut p = dirs_home();
    p.push(".config/sway-power/bar-config.json");
    p
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Edge {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum ModuleKind {
    Workspaces,
    Clock { timezone: String },
    Cpu,
    Memory,
    Tray,
    Bluetooth,
    Network,
    Pulseaudio,
    Launcher { icon: String, tooltip: String, command: String },
    Settings,
    Power,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleConfig {
    pub kind: ModuleKind,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Escape hatch from the bar's global `item_size` for one module.
    #[serde(default)]
    pub size_override: Option<u32>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BarConfig {
    #[serde(default)]
    pub position: Edge,
    #[serde(default = "default_bar_thickness")]
    pub thickness: u32,
    #[serde(default = "default_item_size")]
    pub item_size: u32,
    #[serde(default = "default_bar_spacing")]
    pub spacing: u32,
    #[serde(default = "default_bar_padding")]
    pub padding: u32,
    #[serde(default)]
    pub modules_start: Vec<ModuleConfig>,
    #[serde(default)]
    pub modules_center: Vec<ModuleConfig>,
    #[serde(default)]
    pub modules_end: Vec<ModuleConfig>,
    /// Per-app-id/class icon for the workspaces module — matched
    /// case-insensitively as a substring against a window's app_id or
    /// class, first match wins. User-editable list, not hardcoded
    /// (PLAN.md's own parity checklist calls this out specifically) —
    /// seeded with a sane default here, same as every other field.
    #[serde(default = "default_icon_rewrite")]
    pub icon_rewrite: Vec<IconRewriteRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IconRewriteRule {
    pub match_value: String,
    pub icon: String,
}

fn rule(match_value: &str, icon: &str) -> IconRewriteRule {
    IconRewriteRule { match_value: match_value.to_string(), icon: icon.to_string() }
}

fn default_icon_rewrite() -> Vec<IconRewriteRule> {
    vec![
        rule("og-settings", ""),
        rule("alacritty", "./"),
        rule("brave-browser", "󰖟"),
        rule("firefox", ""),
        rule("chromium", ""),
        rule("code-oss", "󰨞"),
        rule("vesktop", "󰙯"),
        rule("discord", "󰙯"),
        rule("godot", ""),
        rule("factorio", "󰊴"),
        rule("steam_app", "󰊴"),
        rule("steamapp", "󰊴"),
        rule("steam", ""),
        rule("spotify", ""),
        rule("thunar", ""),
        rule("nautilus", ""),
        rule("vlc", "󰕼"),
        rule("gimp", ""),
        rule("obs", ""),
        rule("minecraft", "󰍎"),
    ]
}

fn default_bar_thickness() -> u32 {
    32
}

fn default_item_size() -> u32 {
    32
}

fn default_bar_spacing() -> u32 {
    4
}

fn default_bar_padding() -> u32 {
    4
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            position: Edge::Top,
            thickness: default_bar_thickness(),
            item_size: default_item_size(),
            spacing: default_bar_spacing(),
            padding: default_bar_padding(),
            modules_start: vec![ModuleConfig { kind: ModuleKind::Workspaces, enabled: true, size_override: None }],
            modules_center: vec![
                ModuleConfig { kind: ModuleKind::Cpu, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Memory, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Clock { timezone: String::new() }, enabled: true, size_override: None },
            ],
            modules_end: vec![
                ModuleConfig {
                    kind: ModuleKind::Launcher {
                        icon: "./".to_string(),
                        tooltip: "Open Terminal".to_string(),
                        command: "alacritty".to_string(),
                    },
                    enabled: true,
                    size_override: None,
                },
                ModuleConfig {
                    kind: ModuleKind::Launcher {
                        icon: "".to_string(),
                        tooltip: "Open Claude CLI".to_string(),
                        command: "alacritty -e claude --dangerously-skip-permissions".to_string(),
                    },
                    enabled: true,
                    size_override: None,
                },
                ModuleConfig { kind: ModuleKind::Settings, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Tray, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Bluetooth, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Network, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Pulseaudio, enabled: true, size_override: None },
                ModuleConfig { kind: ModuleKind::Power, enabled: true, size_override: None },
            ],
            icon_rewrite: default_icon_rewrite(),
        }
    }
}

impl BarConfig {
    pub fn load() -> Self {
        let path = bar_config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = bar_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}
