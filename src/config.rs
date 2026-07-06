use serde::Deserialize;

/// Reads the same shared config settings-manager and file-manager already
/// use (`~/.config/sway-power/config.json`) — this app owns none of its
/// own settings, it just picks out the fields it needs. `#[serde(default)]`
/// on everything means it tolerates the file being missing fields (or the
/// whole file missing) without erroring.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_bar_bg")]
    pub bar_bg: String,
    #[serde(default = "default_bar_text")]
    pub bar_text: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_sec_bg")]
    pub sec_bg: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    #[serde(default)]
    pub default_browser: String,
    #[serde(default)]
    pub default_ai_cli: String,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,
    #[serde(default)]
    pub color_variance_enabled: bool,
    #[serde(default = "default_color_variance_amount")]
    pub color_variance_amount: f32,
}

fn default_bar_bg() -> String { "#1a1a2e".into() }
fn default_bar_text() -> String { "#e0e0e0".into() }
fn default_accent() -> String { "#ff7800".into() }
fn default_sec_bg() -> String { "#2a2535".into() }
fn default_terminal() -> String { "alacritty".into() }
fn default_corner_radius() -> f32 { 0.0 }
fn default_color_variance_amount() -> f32 { 0.06 }

impl Default for Config {
    fn default() -> Self {
        Self {
            bar_bg: default_bar_bg(),
            bar_text: default_bar_text(),
            accent: default_accent(),
            sec_bg: default_sec_bg(),
            terminal: default_terminal(),
            default_browser: String::new(),
            default_ai_cli: String::new(),
            corner_radius: default_corner_radius(),
            color_variance_enabled: false,
            color_variance_amount: default_color_variance_amount(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{home}/.config/sway-power/config.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

/// Same per-app tint hash as settings-manager/file-manager/galias — seeded
/// by this app's own name so it always shifts the same way, not random.
pub const APP_TINT_SEED: &str = "app-store";

pub fn apply_color_variance(color: iced::Color, seed: &str, enabled: bool, amount: f32) -> iced::Color {
    if !enabled || amount <= 0.0 {
        return color;
    }
    let hash: u32 = seed.bytes().fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    let t = (hash % 1000) as f32 / 1000.0;
    let shift = (t * 2.0 - 1.0) * amount;
    let clamp = |v: f32| (v + shift).clamp(0.0, 1.0);
    iced::Color { r: clamp(color.r), g: clamp(color.g), b: clamp(color.b), a: color.a }
}
