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
    /// system sleep once enabled. Read by `sway-power-apply`.
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
    #[serde(default = "default_sec_bg")]
    pub sec_bg: String,
    #[serde(default = "default_inactive")]
    pub inactive_color: String,
    #[serde(default = "default_urgent")]
    pub urgent_color: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    /// Used by omegagiven-search for web search / `go/<alias>` lookups.
    #[serde(default)]
    pub default_browser: String,
    /// Used by omegagiven-search's "Ask AI" suggestion.
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
    /// Shared corner-rounding radius (pixels) for settings-manager,
    /// file-manager, and omegagiven-search UI chrome.
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
fn default_clock_12h() -> bool { false }
fn default_corner_radius() -> f32 { 0.0 }
fn default_color_variance_amount() -> f32 { 0.06 }
fn default_cursor_size() -> i32 { 24 }
fn default_wallpaper_mode() -> String { "image".into() }
fn default_wallpaper_color() -> String { "#1a1a2e".into() }
fn default_wallpaper_fit() -> String { "fill".into() }

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
        } else {
            // The waybar config file is hand-edited too (bespoke `custom/*`
            // modules like the terminal/power/clipboard buttons), so on
            // every load absorb anything present live but missing here —
            // otherwise the next module-order write (drag-arrange, a
            // taskbar toggle) would silently delete it by overwriting the
            // bar's module arrays with our stale list. We only ever add,
            // never remove, so this can't fight the drag-arrange screen.
            if let Some((live_l, live_c, live_r)) = read_waybar_module_lists() {
                for (cfg_list, live_list) in [
                    (&mut cfg.modules_left, live_l),
                    (&mut cfg.modules_center, live_c),
                    (&mut cfg.modules_right, live_r),
                ] {
                    for module in live_list {
                        if !cfg_list.contains(&module) {
                            cfg_list.push(module);
                        }
                    }
                }
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
        if cfg.wallpaper_path.is_empty() {
            if let Some((value, is_solid_color, fit)) = read_sway_wallpaper() {
                if is_solid_color {
                    cfg.wallpaper_mode = "color".into();
                    cfg.wallpaper_color = value;
                } else {
                    cfg.wallpaper_mode = "image".into();
                    cfg.wallpaper_path = value;
                    if let Some(fit) = fit {
                        cfg.wallpaper_fit = fit;
                    }
                }
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

/// Returns `(value, is_solid_color, fit_mode)` — value is either the quoted
/// image path or the bare color token, read from the sway config's
/// `output * bg` line. `fit_mode` is the trailing word for image mode
/// (fill/stretch/center/...), None for solid color lines.
fn read_sway_wallpaper() -> Option<(String, bool, Option<String>)> {
    let path = dirs_home().join(".config/sway/config");
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if !t.starts_with("output") || !t.contains(" bg ") {
            continue;
        }
        if let Some(start) = t.find('"') {
            let rest = &t[start + 1..];
            let end = rest.find('"')?;
            let image_path = rest[..end].to_string();
            let fit = rest[end + 1..].split_whitespace().next().map(|s| s.to_string());
            return Some((image_path, false, fit));
        }
        // No quotes: `output * bg <color> solid_color`.
        let rest = t.split_once(" bg ")?.1;
        let color = rest.split_whitespace().next()?;
        return Some((color.to_string(), true, None));
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
