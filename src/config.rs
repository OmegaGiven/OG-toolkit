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
    #[serde(default = "default_sec_bg")]
    pub sec_bg: String,
    #[serde(default = "default_inactive")]
    pub inactive_color: String,
    #[serde(default = "default_urgent")]
    pub urgent_color: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    #[serde(default)]
    pub monitor_configs: Vec<MonitorConfig>,
    #[serde(default = "default_waybar_position")]
    pub waybar_position: String,
    #[serde(default = "default_waybar_thickness")]
    pub waybar_thickness: i32,
    #[serde(default)]
    pub clock_timezone: String,
    #[serde(default)]
    pub extra_clocks: Vec<ClockConfig>,
    #[serde(default)]
    pub next_clock_id: u32,
    /// Module ids per bar section (waybar's `modules-left/center/right`).
    /// Empty until first seeded from the live waybar config — `sway::set_waybar_layout`
    /// leaves a section's array untouched in the bar file when its list here is empty,
    /// so a failed/skipped seed can never wipe out the user's existing bar.
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
}

fn default_border() -> i32 { 1 }
fn default_bar_bg() -> String { "#1a1a2e".into() }
fn default_bar_text() -> String { "#e0e0e0".into() }
fn default_accent() -> String { "#ff7800".into() }
fn default_sec_bg() -> String { "#2a2535".into() }
fn default_inactive() -> String { "#3a3a4a".into() }
fn default_urgent() -> String { "#ff4444".into() }
fn default_terminal() -> String { "alacritty".into() }
fn default_waybar_position() -> String { "left".into() }
fn default_waybar_thickness() -> i32 { 32 }
fn default_cursor_size() -> i32 { 24 }

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_sleep: SleepSetting { enabled: true, minutes: 15 },
            system_sleep: SleepSetting { enabled: false, minutes: 30 },
            gaps_inner: 0,
            gaps_outer: 0,
            border_width: 1,
            bar_bg: default_bar_bg(),
            bar_text: default_bar_text(),
            accent: default_accent(),
            sec_bg: default_sec_bg(),
            inactive_color: default_inactive(),
            urgent_color: default_urgent(),
            terminal: default_terminal(),
            monitor_configs: Vec::new(),
            waybar_position: default_waybar_position(),
            waybar_thickness: default_waybar_thickness(),
            clock_timezone: String::new(),
            extra_clocks: Vec::new(),
            next_clock_id: 0,
            modules_left: Vec::new(),
            modules_center: Vec::new(),
            modules_right: Vec::new(),
            mouse_sensitivity: 0.0,
            cursor_theme: String::new(),
            cursor_size: default_cursor_size(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        let mut cfg: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        if cfg.modules_left.is_empty() && cfg.modules_center.is_empty() && cfg.modules_right.is_empty() {
            if let Some((l, c, r)) = read_waybar_module_lists() {
                cfg.modules_left = l;
                cfg.modules_center = c;
                cfg.modules_right = r;
            }
        }
        if cfg.cursor_theme.is_empty() {
            if let Some((theme, size)) = read_sway_cursor_settings() {
                cfg.cursor_theme = theme;
                cfg.cursor_size = size;
            }
            if let Some(accel) = read_sway_pointer_accel() {
                cfg.mouse_sensitivity = accel;
            }
        }
        cfg
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

fn read_sway_cursor_settings() -> Option<(String, i32)> {
    let path = dirs_home().join(".config/sway/config");
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("seat seat0 xcursor_theme ") {
            let mut parts = rest.split_whitespace();
            let theme = parts.next()?.to_string();
            let size = parts.next().and_then(|s| s.parse().ok()).unwrap_or(default_cursor_size());
            return Some((theme, size));
        }
    }
    None
}

fn read_sway_pointer_accel() -> Option<f32> {
    let path = dirs_home().join(".config/sway/config");
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("input type:pointer pointer_accel ") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn read_waybar_module_lists() -> Option<(Vec<String>, Vec<String>, Vec<String>)> {
    let path = dirs_home().join(".config/waybar/config");
    let content = std::fs::read_to_string(path).ok()?;
    let root: serde_json::Value = serde_json::from_str(&content).ok()?;
    let list = |key: &str| -> Vec<String> {
        root.get(key)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    Some((list("modules-left"), list("modules-center"), list("modules-right")))
}
