use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Local};
use iced::keyboard;
use iced::widget::{button, column, container, row, scrollable, stack, text, text_input, toggler};
use iced::{
    Background, Border, Color, Element, Event, Length, Subscription, Task,
};

use crate::config::Config;
use crate::sway::{self, MonitorInfo};
use crate::tabs::{self, Tab};
use crate::tabs::bar::BarSection;
use crate::tabs::theme::hex_to_color;
use crate::tabs::sysmon;
use og_config::{BarConfig, Edge};

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Local>,
    pub snapshot: Config,
}

/// The name each app seeds its per-app tint hash with when "color variance"
/// is enabled — keep these stable, changing one shifts that app's tint.
const APP_TINT_SEED: &str = "og-settings";

/// `AppColors`, `hex_to_color`, and `apply_color_variance` now live in the
/// shared `og-theme` crate — every OG-toolkit app derives its widget
/// colors from the same definition, so adding a field (like `accent2` or
/// gradient support was) benefits all of them at once instead of needing
/// the same hand-edit copied into each app's `app.rs`.
pub use og_theme::AppColors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSection { Pacman, Aur, Flatpak }

// ── Messages ───────────────────────────────────────────────────────────────

fn search_input_id() -> iced::widget::text_input::Id {
    iced::widget::text_input::Id::new("search-bar")
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(Tab),
    ApplyAndSave,
    Close,
    SearchToggle,
    SearchQueryChanged(String),

    // Notifications
    NotificationsMasterToggled(bool),
    NotifFxEnabledToggled(bool),
    NotifFxEffectSelected(String),
    NotifFxColorChanged(String),
    NotifFxDurationMinus,
    NotifFxDurationPlus,
    NotifFxTestNotification,

    // Power
    MonitorSleepToggled(bool),
    GamescopeSteamToggled(bool),
    SearchFilesToggled(bool),
    SearchSettingsToggled(bool),
    MonitorSleepMinus,
    MonitorSleepPlus,
    SystemSleepToggled(bool),
    SystemSleepMinus,
    SystemSleepPlus,
    ScreenLockToggled(bool),
    ScreenLockMinus,
    ScreenLockPlus,
    StartupExecToggled(usize, bool),
    SystemdServiceToggled(usize, bool),
    SystemServiceToggled(usize, bool),
    PowerServicesLoaded(Vec<sway::StartupExecEntry>, Vec<sway::SystemdServiceInfo>, Vec<sway::SystemdServiceInfo>),
    PowerRowSelected(Option<(u8, usize)>),

    // Network tab
    NetworkDataLoaded(sway::WifiStatus, Vec<sway::WifiNetwork>, Vec<sway::EthernetInterface>, sway::BluetoothAdapter, Vec<sway::BluetoothDevice>),
    WifiPowerToggled(bool),
    WifiScanStart,
    WifiConnect(String),
    WifiDisconnect,
    WifiForget(String),
    EthernetToggled(String, bool),
    BluetoothPowerToggled(bool),
    BluetoothDiscoverableToggled(bool),
    BluetoothPairableToggled(bool),
    BluetoothScanStart,
    BluetoothConnect(String),
    BluetoothDisconnect(String),
    BluetoothPair(String),
    BluetoothRemove(String),

    // Updates tab
    UpdatesStatusLoaded(sway::UpdateStatus),
    UpdatesCheckStart,
    UpdatesApplyPacman(Option<String>),
    UpdatesApplyAur(Option<String>),
    UpdatesApplyFlatpak(Option<String>),
    UpdatesSearchChanged(String),
    UpdatesSectionToggled(UpdateSection),

    // Theme — login screen
    SyncGreeterBackground,

    // Theme — wallpaper
    WallpaperModeChanged(String),
    WallpaperImageSelected(String),
    WallpaperUploadStart,
    WallpaperUploaded(Option<String>),
    WallpaperFitChanged(String),

    // Theme — brightness
    BrightnessMinus,
    BrightnessPlus,
    BrightnessModuleToggled(bool),

    // Display
    GapsInnerMinus,
    GapsInnerPlus,
    GapsOuterMinus,
    GapsOuterPlus,
    BorderWidthMinus,
    BorderWidthPlus,
    MonitorResolutionChanged(String, String),
    MonitorsRefreshed(Vec<MonitorInfo>),

    // Theme — taskbar
    WaybarPositionChanged(String),
    WaybarThicknessMinus,
    WaybarThicknessPlus,
    ClockTimezoneSelected(String),
    Clock12hToggled(bool),
    CornerRadiusMinus,
    CornerRadiusPlus,
    ColorVarianceToggled(bool),
    GradientToggled(bool),
    ColorVarianceAmountMinus,
    ColorVarianceAmountPlus,
    ClockAdd,
    ClockRemove(usize),
    ClockExtraTimezoneSelected(usize, String),

    // Theme — taskbar module arrangement
    ModuleArrangeModeToggle,
    ModuleDragStart(String, f32, f32, f32, f32),  // name, off_x, off_y, start_cursor_x, start_cursor_y
    ModuleDragMove(f32, f32),
    ModuleDragEnd,
    ModuleAdd(String),
    ModuleRemove(String),

    // Bar tab (og-bar's BarConfig)
    BarSetEdge(Edge),
    BarSetThickness(u32),
    BarSetItemSize(u32),
    BarSetSpacing(u32),
    BarSetPadding(u32),
    BarToggleModule(BarSection, usize),
    BarSetClockTimezone(BarSection, usize, String),

    // Hotkeys — bindings
    HotkeyStartCapture(usize),
    HotkeyKeyPressed(String),
    HotkeyKeyEdit(usize, String),
    HotkeyCommandEdit(usize, String),
    HotkeyCommandCommit(usize),
    HotkeyAdd,
    HotkeyRemove(usize),

    // Hotkeys — variables
    HotkeyVarStartCapture(usize),
    HotkeyVarKeyPressed(String),
    HotkeyVarNameEdit(usize, String),
    HotkeyVarValueEdit(usize, String),
    HotkeyVarAdd,
    HotkeyVarRemove(usize),

    // Theme
    ColorChanged(String, String),
    TerminalChanged(String),
    BrowserChanged(String),
    AiCliChanged(String),
    MouseSensitivityChanged(f32),
    UnfocusedOpacityChanged(f32),
    CursorSizeMinus,
    CursorSizePlus,
    CursorThemeChanged(String),
    ColorPickerOpen(String),
    ColorPickerClose,
    ColorWheelChanged(f32, f32),
    ColorPickerValue(f32),
    ColorPickerCommit,
    ThemePreset(String),
    ThemeImport,
    ThemeExport,
    ImportedThemeSelect(usize),
    ThemeSaveAsOpen,
    ThemeSaveAsName(String),
    ThemeSaveAsCancel,
    ThemeSaveAsConfirm,

    // History
    HistoryRestore(usize),

    // Sysmon
    PtyPoll,
    WindowResized(iced::Size),
    TermSizeKnown(iced::Size),
    TermInput(Vec<u8>),

    // Display arrange
    ArrangeModeToggle,
    ArrangeDragStart(String, f32, f32),  // name, offset_x, offset_y (canvas pixels)
    ArrangeDragMove(f32, f32),           // cursor_x, cursor_y (canvas pixels)
    ArrangeDragEnd,
}

// ── App ────────────────────────────────────────────────────────────────────

pub struct App {
    pub config: Config,
    pub saved_config: Config,
    pub bar_config: BarConfig,
    pub current_tab: Tab,
    pub history: Vec<HistoryEntry>,
    // Display
    pub monitors: Vec<MonitorInfo>,

    // Hotkeys
    pub hotkey_bindings: Vec<(String, String)>,
    pub capturing_hotkey: Option<usize>,
    pub hotkey_variables: Vec<(String, String)>,
    pub capturing_variable: Option<usize>,

    // Theme
    pub color_picker_open: Option<String>,
    pub color_picker_h: f32,
    pub color_picker_s: f32,
    pub color_picker_v: f32,
    pub theme_name: String,
    pub imported_themes: Vec<(String, Config)>,
    pub theme_save_open: bool,
    pub theme_save_name: String,
    pub available_terminals: Vec<String>,
    pub available_cursor_themes: Vec<String>,
    pub available_browsers: Vec<String>,
    pub available_ai_clis: Vec<String>,
    pub available_wallpapers: Vec<String>,
    /// False on desktops with no real backlight device — brightness controls
    /// stay visible but disabled so the same build works unmodified on a
    /// laptop with a panel to control.
    pub has_backlight: bool,
    pub brightness: i32,

    // PTY / btop embed
    pub pty_session: Option<crate::pty::PtySession>,
    pub term_canvas: Option<sysmon::TerminalCanvas>,
    pub window_size: iced::Size,
    pub pending_term_resize: Option<std::time::Instant>,

    // Display arrange mode
    pub arrange_mode: bool,
    pub arrange_scale: f32,
    pub arrange_positions: HashMap<String, (f32, f32)>,  // canvas-pixel positions
    pub arrange_dragging: Option<(String, f32, f32)>,    // (name, off_x, off_y)

    // Taskbar module arrange mode
    pub module_arrange_mode: bool,
    pub module_dragging: Option<ModuleDrag>,

    // Power — startup services
    pub startup_execs: Vec<sway::StartupExecEntry>,
    pub systemd_services: Vec<sway::SystemdServiceInfo>,
    pub system_services: Vec<sway::SystemdServiceInfo>,
    /// (section, index) of the currently highlighted row — section 0 =
    /// startup execs, 1 = user services, 2 = system services.
    pub power_selected: Option<(u8, usize)>,

    // Network tab
    pub wifi_status: sway::WifiStatus,
    pub wifi_networks: Vec<sway::WifiNetwork>,
    pub ethernet_interfaces: Vec<sway::EthernetInterface>,
    pub bluetooth_adapter: sway::BluetoothAdapter,
    pub bluetooth_devices: Vec<sway::BluetoothDevice>,
    pub network_scanning: bool,

    // Updates tab
    pub update_status: sway::UpdateStatus,
    pub updates_checking: bool,
    pub updates_search: String,
    pub updates_collapsed: Vec<UpdateSection>,

    // Header search
    pub search_open: bool,
    pub search_query: String,
}

#[derive(Debug, Clone)]
pub struct ModuleDrag {
    pub name: String,
    pub off_x: f32,
    pub off_y: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = crate::config::load_and_seed();
        let saved = config.clone();
        let bar_config = BarConfig::load();
        let hotkeys = load_sway_bindings();
        let variables = load_sway_variables();
        let monitors = sway::get_monitor_info();
        let imported = load_imported_themes();
        let has_backlight = sway::has_backlight();
        let brightness = if has_backlight { sway::get_brightness_percent().unwrap_or(50) as i32 } else { 0 };
        let terminals = detect_terminals(&config.terminal);
        let browsers = detect_browsers(&config.default_browser);
        let ai_clis = detect_ai_clis(&config.default_ai_cli);

        // og-notify always autostarts at login now (no user-facing
        // toggle for that anymore) — the "Enable notification effects"
        // toggle controls whether it's actually running/showing anything.
        sway::set_notification_fx_autostart(true);
        sway::ensure_mako_dnd_mode();
        sway::ensure_opacity_autostart();
        sway::start_opacity_daemon();

        // `--tab <name>` deep-links a fresh launch straight to a tab —
        // used by waybar's network/bluetooth buttons so clicking one goes
        // straight to the Network tab instead of always landing on Power.
        // Only affects a brand-new process; focusing an already-running
        // window doesn't change its current tab (matches how every other
        // "focus-or-launch" waybar button already behaves here).
        let initial_tab = std::env::args()
            .position(|a| a == "--tab")
            .and_then(|i| std::env::args().nth(i + 1))
            .and_then(|name| match name.as_str() {
                "network" => Some(Tab::Network),
                "power" => Some(Tab::Power),
                "display" => Some(Tab::Display),
                "theme" => Some(Tab::Theme),
                "bar" => Some(Tab::Bar),
                "notifications" => Some(Tab::Notifications),
                // Rest added for og-search's new "search settings" results
                // (searches Tab::label() and needs a --tab value for every
                // tab to actually deep-link to it, not just the six that
                // already had one).
                "updates" => Some(Tab::Updates),
                "hotkeys" => Some(Tab::Hotkeys),
                "history" => Some(Tab::History),
                "sysmonitor" => Some(Tab::SysMonitor),
                "search" => Some(Tab::Search),
                _ => None,
            })
            .unwrap_or(Tab::Power);
        let initial_tab_is_network = initial_tab == Tab::Network;

        (
            Self {
                config,
                saved_config: saved,
                bar_config,
                current_tab: initial_tab,
                history: Vec::new(),
                monitors,
                hotkey_bindings: hotkeys,
                capturing_hotkey: None,
                hotkey_variables: variables,
                capturing_variable: None,
                color_picker_open: None,
                color_picker_h: 0.0,
                color_picker_s: 0.0,
                color_picker_v: 0.5,
                theme_name: "Custom".to_string(),
                imported_themes: imported,
                theme_save_open: false,
                theme_save_name: String::new(),
                available_terminals: terminals,
                available_cursor_themes: sway::get_available_cursor_themes(),
                available_browsers: browsers,
                available_ai_clis: ai_clis,
                available_wallpapers: sway::scan_wallpapers(),
                has_backlight,
                brightness,
                pty_session: None,
                term_canvas: None,
                window_size: iced::Size::new(1100.0, 720.0),
                pending_term_resize: None,
                arrange_mode: false,
                arrange_scale: 1.0,
                arrange_positions: HashMap::new(),
                arrange_dragging: None,
                module_arrange_mode: false,
                module_dragging: None,
                // Loaded lazily on first visit to the Power tab (see
                // TabSelected) instead of here — `systemctl list-unit-files`
                // for system-wide units alone takes ~850ms on this machine,
                // which was blocking the window from appearing at all.
                startup_execs: Vec::new(),
                systemd_services: Vec::new(),
                system_services: Vec::new(),
                power_selected: None,
                wifi_status: sway::WifiStatus::default(),
                wifi_networks: Vec::new(),
                ethernet_interfaces: Vec::new(),
                bluetooth_adapter: sway::BluetoothAdapter::default(),
                bluetooth_devices: Vec::new(),
                network_scanning: false,

                update_status: sway::UpdateStatus::default(),
                updates_checking: false,
                updates_search: String::new(),
                updates_collapsed: Vec::new(),
                search_open: false,
                search_query: String::new(),
            },
            if initial_tab_is_network { load_network_data() } else { Task::none() },
        )
    }

    /// Quick, synchronous refresh of Network tab state after a fast action
    /// (power toggle, connect, forget, etc) — unlike scanning, these
    /// individual `rfkill`/`bluetoothctl`/`iwctl` calls return in well under
    /// 100ms, so there's no need for the async `load_network_data` path.
    fn sync_reload_network(&mut self) {
        self.wifi_status = sway::get_wifi_status();
        self.wifi_networks = sway::get_wifi_networks();
        self.ethernet_interfaces = sway::get_ethernet_interfaces();
        self.bluetooth_adapter = sway::get_bluetooth_adapter();
        self.bluetooth_devices = sway::get_bluetooth_devices();
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::TabSelected(t) => {
                // Leaving SysMonitor — kill embedded btop
                if self.current_tab == Tab::SysMonitor && t != Tab::SysMonitor {
                    self.pty_session = None;
                    self.term_canvas = None;
                }
                self.current_tab = t;
                if self.current_tab == Tab::Theme {
                    self.imported_themes = load_imported_themes();
                }
                if self.current_tab == Tab::Power {
                    // `systemctl list-unit-files` for system-wide units alone
                    // takes ~850ms on this machine — run off the UI thread so
                    // switching to this tab doesn't freeze the window.
                    return Task::perform(
                        async {
                            tokio::task::spawn_blocking(|| {
                                (sway::get_startup_execs(), sway::get_systemd_user_services(), sway::get_system_services())
                            })
                            .await
                            .unwrap_or_else(|_| (Vec::new(), Vec::new(), Vec::new()))
                        },
                        |(execs, user, system)| Message::PowerServicesLoaded(execs, user, system),
                    );
                }
                if self.current_tab == Tab::Network {
                    return load_network_data();
                }
                if self.current_tab == Tab::Updates {
                    return load_update_status();
                }
                // Entering SysMonitor — ask the runtime for the real window
                // size first; self.window_size can be stale if no resize
                // event fired yet. btop is spawned in TermSizeKnown.
                if self.current_tab == Tab::SysMonitor && self.pty_session.is_none() {
                    return iced::window::get_latest()
                        .and_then(iced::window::get_size)
                        .map(Message::TermSizeKnown);
                }
            }

            Message::TermSizeKnown(size) => {
                self.window_size = size;
                if self.current_tab == Tab::SysMonitor && self.pty_session.is_none() {
                    let (rows, cols) = sysmon::grid_for(size);
                    if let Ok(session) = crate::pty::spawn_btop(rows, cols) {
                        let canvas = sysmon::TerminalCanvas::new(
                            Arc::clone(&session.parser),
                            Arc::clone(&session.writer),
                        );
                        self.term_canvas = Some(canvas);
                        self.pty_session = Some(session);
                    }
                }
            }

            Message::TermInput(bytes) => {
                if let Some(session) = self.pty_session.as_ref() {
                    crate::pty::send_input(&session.writer, &bytes);
                }
            }

            Message::ApplyAndSave => {
                let _ = self.config.save();
                let _ = self.bar_config.save();
                self.history.push(HistoryEntry {
                    timestamp: Local::now(),
                    snapshot: self.config.clone(),
                });
                self.saved_config = self.config.clone();
                sway::apply_theme(&self.config);
                save_to_sway_config(&self.hotkey_variables, &self.hotkey_bindings);
                crate::terminal_theme::apply_terminal_theme(&self.config);
            }

            Message::Close => {
                return iced::exit();
            }
            Message::SearchToggle => {
                self.search_open = !self.search_open;
                if !self.search_open {
                    self.search_query.clear();
                } else {
                    return iced::widget::text_input::focus(search_input_id());
                }
            }
            Message::SearchQueryChanged(q) => { self.search_query = q; }

            // Power
            Message::MonitorSleepToggled(v) => { self.config.monitor_sleep.enabled = v; }
            Message::GamescopeSteamToggled(v) => {
                self.config.gamescope_steam = v;
                let _ = self.config.save();
                sway::apply_gamescope_steam(v);
            }
            Message::SearchFilesToggled(v) => {
                self.config.search_files_enabled = v;
                let _ = self.config.save();
            }
            Message::SearchSettingsToggled(v) => {
                self.config.search_settings_enabled = v;
                let _ = self.config.save();
            }
            Message::MonitorSleepMinus => {
                self.config.monitor_sleep.minutes = self.config.monitor_sleep.minutes.saturating_sub(1).max(1);
            }
            Message::MonitorSleepPlus => { self.config.monitor_sleep.minutes += 1; }
            Message::SystemSleepToggled(v) => { self.config.system_sleep.enabled = v; }
            Message::SystemSleepMinus => {
                self.config.system_sleep.minutes = self.config.system_sleep.minutes.saturating_sub(1).max(1);
            }
            Message::SystemSleepPlus => { self.config.system_sleep.minutes += 1; }
            Message::ScreenLockToggled(v) => { self.config.screen_lock.enabled = v; }
            Message::ScreenLockMinus => {
                self.config.screen_lock.minutes = self.config.screen_lock.minutes.saturating_sub(1).max(1);
            }
            Message::ScreenLockPlus => { self.config.screen_lock.minutes += 1; }
            Message::StartupExecToggled(idx, enabled) => {
                if let Some(e) = self.startup_execs.get_mut(idx) {
                    if let Some(new_line) = sway::toggle_startup_exec(e.line_index, &e.original_line, enabled) {
                        e.original_line = new_line;
                        e.enabled = enabled;
                    }
                }
            }
            Message::SystemdServiceToggled(idx, enabled) => {
                if let Some(s) = self.systemd_services.get_mut(idx) {
                    sway::set_systemd_service_enabled(&s.unit, enabled);
                    s.enabled = enabled;
                }
            }
            Message::SystemServiceToggled(idx, enabled) => {
                if let Some(s) = self.system_services.get_mut(idx) {
                    sway::set_system_service_enabled(&self.config.terminal, &s.unit, enabled);
                    s.enabled = enabled;
                }
            }
            Message::PowerServicesLoaded(execs, user, system) => {
                self.startup_execs = execs;
                self.systemd_services = user;
                self.system_services = system;
            }
            Message::PowerRowSelected(sel) => {
                self.power_selected = if self.power_selected == sel { None } else { sel };
            }

            // Network tab
            Message::NetworkDataLoaded(wifi, networks, eth, bt_adapter, bt_devices) => {
                self.wifi_status = wifi;
                self.wifi_networks = networks;
                self.ethernet_interfaces = eth;
                self.bluetooth_adapter = bt_adapter;
                self.bluetooth_devices = bt_devices;
                self.network_scanning = false;
            }
            Message::WifiPowerToggled(on) => {
                sway::set_wifi_power(on);
                self.sync_reload_network();
            }
            Message::WifiScanStart => {
                self.network_scanning = true;
                return Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| {
                            sway::wifi_scan();
                            (
                                sway::get_wifi_status(),
                                sway::get_wifi_networks(),
                                sway::get_ethernet_interfaces(),
                                sway::get_bluetooth_adapter(),
                                sway::get_bluetooth_devices(),
                            )
                        })
                        .await
                        .unwrap_or_else(|_| (
                            sway::WifiStatus::default(), Vec::new(), Vec::new(),
                            sway::BluetoothAdapter::default(), Vec::new(),
                        ))
                    },
                    |(wifi, networks, eth, bt_adapter, bt_devices)| {
                        Message::NetworkDataLoaded(wifi, networks, eth, bt_adapter, bt_devices)
                    },
                );
            }
            Message::WifiConnect(ssid) => {
                sway::wifi_connect(&self.config.terminal, &ssid);
                self.sync_reload_network();
            }
            Message::WifiDisconnect => {
                sway::wifi_disconnect();
                self.sync_reload_network();
            }
            Message::WifiForget(ssid) => {
                sway::wifi_forget(&ssid);
                self.sync_reload_network();
            }
            Message::EthernetToggled(iface, up) => {
                sway::set_ethernet_up(&self.config.terminal, &iface, up);
                self.sync_reload_network();
            }
            Message::BluetoothPowerToggled(on) => {
                sway::set_bluetooth_power(on);
                self.sync_reload_network();
            }
            Message::BluetoothDiscoverableToggled(on) => {
                sway::set_bluetooth_discoverable(on);
                self.sync_reload_network();
            }
            Message::BluetoothPairableToggled(on) => {
                sway::set_bluetooth_pairable(on);
                self.sync_reload_network();
            }
            Message::BluetoothScanStart => {
                self.network_scanning = true;
                return Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| {
                            sway::bluetooth_scan();
                            (
                                sway::get_wifi_status(),
                                sway::get_wifi_networks(),
                                sway::get_ethernet_interfaces(),
                                sway::get_bluetooth_adapter(),
                                sway::get_bluetooth_devices(),
                            )
                        })
                        .await
                        .unwrap_or_else(|_| (
                            sway::WifiStatus::default(), Vec::new(), Vec::new(),
                            sway::BluetoothAdapter::default(), Vec::new(),
                        ))
                    },
                    |(wifi, networks, eth, bt_adapter, bt_devices)| {
                        Message::NetworkDataLoaded(wifi, networks, eth, bt_adapter, bt_devices)
                    },
                );
            }
            Message::BluetoothConnect(mac) => {
                sway::bluetooth_connect(&mac);
                self.sync_reload_network();
            }
            Message::BluetoothDisconnect(mac) => {
                sway::bluetooth_disconnect(&mac);
                self.sync_reload_network();
            }
            Message::BluetoothPair(mac) => {
                sway::bluetooth_pair(&self.config.terminal, &mac);
                self.sync_reload_network();
            }
            Message::BluetoothRemove(mac) => {
                sway::bluetooth_remove(&mac);
                self.sync_reload_network();
            }

            // Updates tab
            Message::UpdatesStatusLoaded(status) => {
                self.update_status = status;
                self.updates_checking = false;
            }
            Message::UpdatesCheckStart => {
                self.updates_checking = true;
                return load_update_status();
            }
            Message::UpdatesApplyPacman(pkg) => sway::update_pacman(&self.config.terminal, pkg.as_deref()),
            Message::UpdatesApplyAur(pkg) => sway::update_aur(&self.config.terminal, pkg.as_deref()),
            Message::UpdatesApplyFlatpak(id) => sway::update_flatpak(&self.config.terminal, id.as_deref()),
            Message::UpdatesSearchChanged(q) => self.updates_search = q,
            Message::UpdatesSectionToggled(section) => {
                if let Some(pos) = self.updates_collapsed.iter().position(|s| *s == section) {
                    self.updates_collapsed.remove(pos);
                } else {
                    self.updates_collapsed.push(section);
                }
            }

            Message::SyncGreeterBackground => {
                if let Some(wallpaper) = sway::get_current_wallpaper() {
                    sway::sync_greeter_background(&self.config.terminal, &wallpaper);
                }
            }

            Message::WallpaperFitChanged(fit) => { self.config.wallpaper_fit = fit; }
            Message::BrightnessMinus => {
                self.brightness = (self.brightness - 5).max(0);
                sway::set_brightness_percent(self.brightness as u32);
            }
            Message::BrightnessPlus => {
                self.brightness = (self.brightness + 5).min(100);
                sway::set_brightness_percent(self.brightness as u32);
            }
            Message::BrightnessModuleToggled(show) => {
                if show {
                    if !self.config.modules_right.iter().any(|m| m == "backlight") {
                        self.config.modules_right.push("backlight".to_string());
                    }
                } else {
                    self.config.modules_right.retain(|m| m != "backlight");
                }
                sway::set_waybar_layout(&self.config);
            }
            Message::WallpaperModeChanged(mode) => { self.config.wallpaper_mode = mode; }
            Message::WallpaperImageSelected(path) => { self.config.wallpaper_path = path; }
            Message::WallpaperUploadStart => {
                return Task::perform(
                    async { tokio::task::spawn_blocking(sway::import_wallpaper).await.unwrap_or(None) },
                    Message::WallpaperUploaded,
                );
            }
            Message::WallpaperUploaded(picked) => {
                if let Some(path) = picked {
                    self.config.wallpaper_mode = "image".to_string();
                    self.config.wallpaper_path = path.clone();
                    if !self.available_wallpapers.contains(&path) {
                        self.available_wallpapers.push(path);
                        self.available_wallpapers.sort();
                    }
                }
            }

            // Display
            Message::GapsInnerMinus => { self.config.gaps_inner = (self.config.gaps_inner - 1).max(0); }
            Message::GapsInnerPlus => { self.config.gaps_inner += 1; }
            Message::GapsOuterMinus => { self.config.gaps_outer = (self.config.gaps_outer - 1).max(0); }
            Message::GapsOuterPlus => { self.config.gaps_outer += 1; }
            Message::BorderWidthMinus => { self.config.border_width = (self.config.border_width - 1).max(0); }
            Message::BorderWidthPlus => { self.config.border_width += 1; }
            Message::MonitorResolutionChanged(name, res) => {
                sway::set_monitor_mode(&name, &res);
                if let Some(mc) = self.config.monitor_configs.iter_mut().find(|m| m.name == name) {
                    mc.resolution = res;
                } else {
                    self.config.monitor_configs.push(crate::config::MonitorConfig {
                        name,
                        resolution: res,
                        x: 0,
                        y: 0,
                        enabled: true,
                    });
                }
            }
            Message::MonitorsRefreshed(m) => { self.monitors = m; }
            Message::WaybarPositionChanged(pos) => {
                self.config.waybar_position = pos;
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::WaybarThicknessMinus => {
                self.config.waybar_thickness = (self.config.waybar_thickness - 2).max(16);
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::WaybarThicknessPlus => {
                self.config.waybar_thickness += 2;
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::ClockTimezoneSelected(tz) => {
                self.config.clock_timezone = if tz == "System Default" { String::new() } else { tz };
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::Clock12hToggled(v) => {
                self.config.clock_12h = v;
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::CornerRadiusMinus => {
                self.config.corner_radius = (self.config.corner_radius - 2.0).max(0.0);
            }
            Message::CornerRadiusPlus => {
                self.config.corner_radius += 2.0;
            }
            Message::ColorVarianceToggled(v) => { self.config.color_variance_enabled = v; }
            Message::GradientToggled(v) => { self.config.gradient_enabled = v; }
            Message::ColorVarianceAmountMinus => {
                self.config.color_variance_amount = (self.config.color_variance_amount - 0.01).max(0.0);
            }
            Message::ColorVarianceAmountPlus => {
                self.config.color_variance_amount = (self.config.color_variance_amount + 0.01).min(0.3);
            }
            Message::ClockAdd => {
                let id = self.config.next_clock_id;
                self.config.next_clock_id += 1;
                self.config.extra_clocks.push(crate::config::ClockConfig { id, timezone: "UTC".to_string() });
                // Only auto-place it if arrangement has actually been seeded —
                // an empty modules_right means "never touched", and set_waybar_layout
                // leaves an empty section alone rather than overwriting it.
                if !self.config.modules_right.is_empty() {
                    self.config.modules_right.push(format!("clock#{id}"));
                }
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }
            Message::ClockRemove(idx) => {
                if idx < self.config.extra_clocks.len() {
                    let removed = self.config.extra_clocks.remove(idx);
                    let removed_id = removed.module_id();
                    for list in [&mut self.config.modules_left, &mut self.config.modules_center, &mut self.config.modules_right] {
                        list.retain(|m| m != &removed_id);
                    }
                    let _ = self.config.save();
                    sway::set_waybar_layout(&self.config);
                }
            }
            Message::ClockExtraTimezoneSelected(idx, tz) => {
                if let Some(entry) = self.config.extra_clocks.get_mut(idx) {
                    entry.timezone = tz;
                    let _ = self.config.save();
                    sway::set_waybar_layout(&self.config);
                }
            }

            Message::ModuleArrangeModeToggle => {
                self.module_arrange_mode = !self.module_arrange_mode;
                self.module_dragging = None;
            }
            Message::ModuleDragStart(name, off_x, off_y, start_x, start_y) => {
                self.module_dragging = Some(ModuleDrag { name, off_x, off_y, cursor_x: start_x, cursor_y: start_y });
            }
            Message::ModuleDragMove(cx, cy) => {
                if let Some(d) = self.module_dragging.as_mut() {
                    d.cursor_x = cx;
                    d.cursor_y = cy;
                }
            }
            Message::ModuleDragEnd => {
                if let Some(d) = self.module_dragging.take() {
                    let (section, idx) = tabs::taskbar_arrange::drop_target(&self.config, d.cursor_x, d.cursor_y);
                    for list in [&mut self.config.modules_left, &mut self.config.modules_center, &mut self.config.modules_right] {
                        list.retain(|m| m != &d.name);
                    }
                    let target = match section {
                        0 => &mut self.config.modules_left,
                        1 => &mut self.config.modules_center,
                        _ => &mut self.config.modules_right,
                    };
                    let idx = idx.min(target.len());
                    target.insert(idx, d.name);
                    let _ = self.config.save();
                    sway::set_waybar_layout(&self.config);
                }
            }
            Message::ModuleAdd(id) => {
                let already_placed = [&self.config.modules_left, &self.config.modules_center, &self.config.modules_right]
                    .into_iter().any(|list| list.contains(&id));
                if !already_placed {
                    self.config.modules_right.push(id);
                    let _ = self.config.save();
                    sway::set_waybar_layout(&self.config);
                }
            }
            Message::ModuleRemove(id) => {
                for list in [&mut self.config.modules_left, &mut self.config.modules_center, &mut self.config.modules_right] {
                    list.retain(|m| m != &id);
                }
                let _ = self.config.save();
                sway::set_waybar_layout(&self.config);
            }

            // Bar tab — staged in-memory, written on ApplyAndSave like
            // every other tab (unlike og-bar's own popout, which auto-saves
            // every change instantly).
            Message::BarSetEdge(edge) => { self.bar_config.position = edge; }
            Message::BarSetThickness(v) => { self.bar_config.thickness = v; }
            Message::BarSetItemSize(v) => { self.bar_config.item_size = v; }
            Message::BarSetSpacing(v) => { self.bar_config.spacing = v; }
            Message::BarSetPadding(v) => { self.bar_config.padding = v; }
            Message::BarToggleModule(section, index) => {
                tabs::bar::toggle_module(&mut self.bar_config, section, index);
            }
            Message::BarSetClockTimezone(section, index, tz) => {
                tabs::bar::set_clock_timezone(&mut self.bar_config, section, index, tz);
            }

            // Hotkeys — bindings
            Message::HotkeyStartCapture(idx) => {
                self.capturing_variable = None;
                self.capturing_hotkey = Some(idx);
            }
            Message::HotkeyKeyPressed(key_str) => {
                if let Some(idx) = self.capturing_hotkey {
                    let is_modifier_only = matches!(
                        key_str.trim_start_matches("$mod+")
                            .trim_start_matches("ctrl+")
                            .trim_start_matches("alt+")
                            .trim_start_matches("shift+"),
                        "" | "shift" | "ctrl" | "alt" | "meta" | "super" | "hyper"
                    );
                    if !is_modifier_only && !key_str.is_empty() {
                        if let Some(binding) = self.hotkey_bindings.get_mut(idx) {
                            binding.0 = key_str;
                        }
                        self.capturing_hotkey = None;
                    }
                }
            }
            Message::HotkeyKeyEdit(idx, val) => {
                if let Some(binding) = self.hotkey_bindings.get_mut(idx) {
                    binding.0 = val;
                }
            }
            Message::HotkeyCommandEdit(idx, val) => {
                if let Some(binding) = self.hotkey_bindings.get_mut(idx) {
                    binding.1 = val;
                }
            }
            Message::HotkeyCommandCommit(_) => {}
            Message::HotkeyAdd => {
                self.hotkey_bindings.push(("".into(), "".into()));
            }
            Message::HotkeyRemove(idx) => {
                if idx < self.hotkey_bindings.len() {
                    self.hotkey_bindings.remove(idx);
                }
            }

            // Hotkeys — variables
            Message::HotkeyVarStartCapture(idx) => {
                self.capturing_hotkey = None;
                self.capturing_variable = Some(idx);
            }
            Message::HotkeyVarKeyPressed(mod_name) => {
                if let Some(idx) = self.capturing_variable {
                    if let Some(var) = self.hotkey_variables.get_mut(idx) {
                        var.1 = mod_name;
                    }
                    self.capturing_variable = None;
                }
            }
            Message::HotkeyVarNameEdit(idx, val) => {
                if let Some(var) = self.hotkey_variables.get_mut(idx) {
                    var.0 = val;
                }
            }
            Message::HotkeyVarValueEdit(idx, val) => {
                if let Some(var) = self.hotkey_variables.get_mut(idx) {
                    var.1 = val;
                }
            }
            Message::HotkeyVarAdd => {
                self.hotkey_variables.push(("$".into(), "".into()));
            }
            Message::HotkeyVarRemove(idx) => {
                if idx < self.hotkey_variables.len() {
                    self.hotkey_variables.remove(idx);
                }
            }

            // Theme
            Message::ColorChanged(key, val) => {
                if matches!(self.theme_name.as_str(), "Dark" | "Light") {
                    self.theme_name = "Custom".to_string();
                }
                match key.as_str() {
                    "bar_bg" => self.config.bar_bg = val,
                    "sec_bg" => self.config.sec_bg = val,
                    "bar_text" => self.config.bar_text = val,
                    "accent" => self.config.accent = val,
                    "accent2" => self.config.accent2 = val,
                    "inactive_color" => self.config.inactive_color = val,
                    "urgent_color" => self.config.urgent_color = val,
                    "wallpaper_color" => self.config.wallpaper_color = val,
                    _ => {}
                }
            }
            Message::TerminalChanged(v) => { self.config.terminal = v; }
            Message::BrowserChanged(v) => { self.config.default_browser = v; }
            Message::AiCliChanged(v) => { self.config.default_ai_cli = v; }
            Message::MouseSensitivityChanged(v) => {
                self.config.mouse_sensitivity = v;
                let _ = self.config.save();
                sway::set_mouse_sensitivity(v);
            }
            Message::UnfocusedOpacityChanged(v) => {
                self.config.unfocused_opacity = v;
                let _ = self.config.save();
                sway::apply_unfocused_opacity_now(v);
            }
            Message::CursorSizeMinus => {
                self.config.cursor_size = (self.config.cursor_size - 4).max(8);
                let _ = self.config.save();
                sway::set_cursor(&self.config.cursor_theme, self.config.cursor_size);
            }
            Message::CursorSizePlus => {
                self.config.cursor_size += 4;
                let _ = self.config.save();
                sway::set_cursor(&self.config.cursor_theme, self.config.cursor_size);
            }
            Message::CursorThemeChanged(theme) => {
                self.config.cursor_theme = theme;
                let _ = self.config.save();
                sway::set_cursor(&self.config.cursor_theme, self.config.cursor_size);
            }
            Message::NotifFxEnabledToggled(v) => {
                self.config.notif_fx_enabled = v;
                let _ = self.config.save();
                if v {
                    sway::start_notification_fx();
                } else {
                    sway::stop_notification_fx();
                }
            }
            Message::NotifFxEffectSelected(v) => {
                self.config.notif_fx_effect = v;
                let _ = self.config.save();
            }
            Message::NotifFxColorChanged(v) => {
                self.config.notif_fx_color = v;
                let _ = self.config.save();
            }
            Message::NotifFxDurationMinus => {
                self.config.notif_fx_duration_ms = self.config.notif_fx_duration_ms.saturating_sub(200).max(400);
                let _ = self.config.save();
            }
            Message::NotifFxDurationPlus => {
                self.config.notif_fx_duration_ms = (self.config.notif_fx_duration_ms + 200).min(4000);
                let _ = self.config.save();
            }
            Message::NotificationsMasterToggled(v) => {
                sway::set_system_notifications_enabled(v);
            }
            Message::NotifFxTestNotification => {
                let home = crate::config::dirs_home();
                let _ = std::process::Command::new(home.join(".local/bin/og-notify"))
                    .arg("--preview")
                    .spawn();
                let _ = std::process::Command::new("notify-send")
                    .args(["OG Settings", "Notification effect test"])
                    .spawn();
            }
            Message::ColorPickerOpen(key) => {
                let current_hex = match key.as_str() {
                    "bar_bg" => &self.config.bar_bg,
                    "sec_bg" => &self.config.sec_bg,
                    "bar_text" => &self.config.bar_text,
                    "accent" => &self.config.accent,
                    "accent2" => &self.config.accent2,
                    "inactive_color" => &self.config.inactive_color,
                    "urgent_color" => &self.config.urgent_color,
                    "wallpaper_color" => &self.config.wallpaper_color,
                    _ => &self.config.bar_bg,
                };
                let c = hex_to_color(current_hex);
                let (h, s, v) = crate::color_wheel::rgb_to_hsv(c);
                self.color_picker_h = h;
                self.color_picker_s = s;
                self.color_picker_v = v;
                self.color_picker_open = Some(key);
            }
            Message::ColorPickerClose => {
                self.color_picker_open = None;
            }
            Message::ColorWheelChanged(h, s) => {
                self.color_picker_h = h;
                self.color_picker_s = s;
            }
            Message::ColorPickerValue(v) => { self.color_picker_v = v; }
            Message::ColorPickerCommit => {
                if let Some(key) = self.color_picker_open.take() {
                    let rgb = crate::color_wheel::hsv_to_rgb(self.color_picker_h, self.color_picker_s, self.color_picker_v);
                    let hex = format!(
                        "#{:02x}{:02x}{:02x}",
                        (rgb.r * 255.0).round() as u8,
                        (rgb.g * 255.0).round() as u8,
                        (rgb.b * 255.0).round() as u8,
                    );
                    if matches!(self.theme_name.as_str(), "Dark" | "Light") {
                        self.theme_name = "Custom".to_string();
                    }
                    match key.as_str() {
                        "bar_bg" => self.config.bar_bg = hex,
                        "sec_bg" => self.config.sec_bg = hex,
                        "bar_text" => self.config.bar_text = hex,
                        "accent" => self.config.accent = hex,
                        "accent2" => self.config.accent2 = hex,
                        "inactive_color" => self.config.inactive_color = hex,
                        "urgent_color" => self.config.urgent_color = hex,
                        "wallpaper_color" => self.config.wallpaper_color = hex,
                        _ => {}
                    }
                }
            }
            Message::ThemePreset(name) => {
                let preset = if name == "Dark" { dark_preset() } else { light_preset() };
                self.config.bar_bg = preset.bar_bg;
                self.config.sec_bg = preset.sec_bg;
                self.config.bar_text = preset.bar_text;
                self.config.accent = preset.accent;
                self.config.inactive_color = preset.inactive_color;
                self.config.urgent_color = preset.urgent_color;
                self.theme_name = name;
            }
            Message::ThemeImport => {
                self.imported_themes = load_imported_themes();
            }
            Message::ThemeExport => {
                let home = std::env::var("HOME").unwrap_or_default();
                let themes_dir = format!("{}/themes", home);
                let _ = std::fs::create_dir_all(&themes_dir);
                let path = format!("{}/{}.json", themes_dir, sanitize_name(&self.theme_name));
                if let Ok(json) = serde_json::to_string_pretty(&self.config) {
                    let _ = std::fs::write(&path, json);
                }
                self.imported_themes = load_imported_themes();
            }
            Message::ThemeSaveAsOpen => {
                self.theme_save_open = true;
                self.theme_save_name.clear();
            }
            Message::ThemeSaveAsName(name) => {
                self.theme_save_name = name;
            }
            Message::ThemeSaveAsCancel => {
                self.theme_save_open = false;
            }
            Message::ThemeSaveAsConfirm => {
                let name = self.theme_save_name.trim().to_string();
                if !name.is_empty() {
                    let home = std::env::var("HOME").unwrap_or_default();
                    let themes_dir = format!("{}/themes", home);
                    let _ = std::fs::create_dir_all(&themes_dir);
                    let path = format!("{}/{}.json", themes_dir, sanitize_name(&name));
                    if let Ok(json) = serde_json::to_string_pretty(&self.config) {
                        let _ = std::fs::write(&path, json);
                    }
                    self.imported_themes = load_imported_themes();
                    self.theme_name = sanitize_name(&name);
                    self.theme_save_open = false;
                }
            }
            Message::ImportedThemeSelect(idx) => {
                if let Some((name, cfg)) = self.imported_themes.get(idx) {
                    let name = name.clone();
                    self.config.bar_bg = cfg.bar_bg.clone();
                    self.config.sec_bg = cfg.sec_bg.clone();
                    self.config.bar_text = cfg.bar_text.clone();
                    self.config.accent = cfg.accent.clone();
                    self.config.inactive_color = cfg.inactive_color.clone();
                    self.config.urgent_color = cfg.urgent_color.clone();
                    self.theme_name = name;
                }
            }

            // History
            Message::HistoryRestore(i) => {
                if let Some(entry) = self.history.get(i) {
                    self.config = entry.snapshot.clone();
                }
            }

            // Sysmon — PtyPoll fires every 50ms to keep iced redrawing the terminal canvas
            Message::PtyPoll => {
                // Debounced respawn after a resize: btop redraws cleanly at the
                // new size, which live pty resizing does not (vt100 alt-screen
                // keeps stale content).
                if let Some(t) = self.pending_term_resize {
                    if t.elapsed() >= std::time::Duration::from_millis(300) {
                        self.pending_term_resize = None;
                        let (rows, cols) = sysmon::grid_for(self.window_size);
                        if let Ok(session) = crate::pty::spawn_btop(rows, cols) {
                            self.term_canvas = Some(sysmon::TerminalCanvas::new(
                                Arc::clone(&session.parser),
                                Arc::clone(&session.writer),
                            ));
                            self.pty_session = Some(session);
                        }
                    }
                }
            }

            Message::WindowResized(size) => {
                self.window_size = size;
                if self.current_tab == Tab::SysMonitor {
                    if let Some(session) = self.pty_session.as_ref() {
                        let (rows, cols) = sysmon::grid_for(size);
                        if rows != session.rows || cols != session.cols {
                            self.pending_term_resize = Some(std::time::Instant::now());
                        }
                    }
                }
            }

            // Display arrange
            Message::ArrangeModeToggle => {
                if self.arrange_mode {
                    // Exiting: apply stored positions to swaymsg
                    for m in &self.monitors {
                        let scale = self.arrange_scale;
                        let (cx, cy) = self.arrange_positions.get(&m.name)
                            .copied()
                            .unwrap_or((m.x as f32 * scale + 10.0, m.y as f32 * scale + 10.0));
                        let lx = ((cx - 10.0) / scale).round() as i32;
                        let ly = ((cy - 10.0) / scale).round() as i32;
                        let _ = std::process::Command::new("swaymsg")
                            .args(["output", &m.name, "pos", &lx.max(0).to_string(), &ly.max(0).to_string()])
                            .spawn();
                        if let Some(mc) = self.config.monitor_configs.iter_mut().find(|mc| mc.name == m.name) {
                            mc.x = lx.max(0);
                            mc.y = ly.max(0);
                        }
                    }
                    self.arrange_mode = false;
                    self.arrange_positions.clear();
                    self.arrange_dragging = None;
                    self.monitors = sway::get_monitor_info();
                } else {
                    // Entering: compute scale, init positions
                    let max_x = self.monitors.iter().map(|m| m.x + m.width).max().unwrap_or(1920).max(1) as f32;
                    let max_y = self.monitors.iter().map(|m| m.y + m.height).max().unwrap_or(1080).max(1) as f32;
                    self.arrange_scale = (660.0f32 / max_x).min(140.0 / max_y);
                    self.arrange_positions.clear();
                    // Seed positions from current monitor layout
                    for m in &self.monitors {
                        let cx = m.x as f32 * self.arrange_scale + 10.0;
                        let cy = m.y as f32 * self.arrange_scale + 10.0;
                        self.arrange_positions.insert(m.name.clone(), (cx, cy));
                    }
                    self.arrange_mode = true;
                }
            }
            Message::ArrangeDragStart(name, off_x, off_y) => {
                self.arrange_dragging = Some((name, off_x, off_y));
            }
            Message::ArrangeDragMove(cx, cy) => {
                if let Some((ref name, off_x, off_y)) = self.arrange_dragging {
                    let new_x = (cx - off_x).max(0.0);
                    let new_y = (cy - off_y).max(0.0);
                    self.arrange_positions.insert(name.clone(), (new_x, new_y));
                }
            }
            Message::ArrangeDragEnd => {
                if let Some((ref name, _, _)) = self.arrange_dragging.clone() {
                    self.snap_to_edges(name.clone());
                }
                self.arrange_dragging = None;
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let colors = AppColors::from_config(&self.config, APP_TINT_SEED);
        let accent = colors.accent;

        let searching = self.search_open && !self.search_query.trim().is_empty();

        // SysMonitor renders a fill-height terminal canvas; inside a scrollable
        // its height would resolve to zero, so it gets the space directly.
        let content: Element<Message> = if searching {
            scrollable(self.search_results_view(colors)).width(Length::Fill).height(Length::Fill).into()
        } else if self.current_tab == Tab::SysMonitor {
            container(self.tab_content(colors)).width(Length::Fill).height(Length::Fill).into()
        } else {
            scrollable(self.tab_content(colors)).width(Length::Fill).height(Length::Fill).into()
        };

        let body: Element<Message> = row![self.sidebar(colors), content].height(Length::Fill).into();

        let main_content: Element<Message> = column![
            self.header(colors),
            accent_line(accent),
            body,
        ]
        .into();

        let base = container(main_content)
            .style(move |_| container::Style {
                background: Some(colors.bg_fill),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill);

        // Modal overlays
        if self.color_picker_open.is_some() {
            let modal = self.color_picker_modal(colors);
            iced::widget::stack![base, modal].into()
        } else if self.theme_save_open {
            let modal = self.theme_save_modal(colors);
            iced::widget::stack![base, modal].into()
        } else {
            base.into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let monitor_timer = iced::time::every(std::time::Duration::from_secs(5))
            .map(|_| Message::MonitorsRefreshed(sway::get_monitor_info()));

        let pty_poll = if self.current_tab == Tab::SysMonitor && self.pty_session.is_some() {
            Some(
                iced::time::every(std::time::Duration::from_millis(50))
                    .map(|_| Message::PtyPoll),
            )
        } else {
            None
        };

        let kb = if self.capturing_hotkey.is_some() {
            Some(iced::event::listen_with(|event, _status, _id| {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                    Some(Message::HotkeyKeyPressed(format_key(&key, modifiers)))
                } else {
                    None
                }
            }))
        } else if self.capturing_variable.is_some() {
            Some(iced::event::listen_with(|event, _status, _id| {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                    if let Some(mod_name) = format_modifier_key(&key, modifiers) {
                        Some(Message::HotkeyVarKeyPressed(mod_name))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }))
        } else {
            None
        };

        let resize_events = iced::window::resize_events()
            .map(|(_id, size)| Message::WindowResized(size));

        // Forward keyboard to the embedded terminal while it is showing
        // (unless a hotkey capture is in progress).
        let term_kb = if self.current_tab == Tab::SysMonitor
            && self.pty_session.is_some()
            && self.capturing_hotkey.is_none()
            && self.capturing_variable.is_none()
        {
            Some(iced::event::listen_with(|event, _status, _id| {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) = event {
                    term_key_bytes(&key, text.as_deref()).map(Message::TermInput)
                } else {
                    None
                }
            }))
        } else {
            None
        };

        let mut subs = vec![monitor_timer, resize_events];
        if let Some(p) = pty_poll { subs.push(p); }
        if let Some(k) = kb { subs.push(k); }
        if let Some(t) = term_kb { subs.push(t); }
        Subscription::batch(subs)
    }

    // ── Header ────────────────────────────────────────────────────────────

    fn header<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        let c = colors;

        let hdr_btn = move |label: &'static str, msg: Message, enabled: bool| -> Element<'a, Message> {
            button(
                text(label).style(move |_| iced::widget::text::Style {
                    color: Some(if enabled { c.text } else { Color { a: 0.35, ..c.text } }),
                })
            )
            .style(move |_, status| {
                let is_disabled = matches!(status, iced::widget::button::Status::Disabled);
                let bg = if is_disabled {
                    Color { a: 0.3, ..c.header_btn_bg }
                } else {
                    match status {
                        iced::widget::button::Status::Hovered => c.surface,
                        _ => c.header_btn_bg,
                    }
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: if is_disabled { Color { a: 0.35, ..c.text } } else { c.text },
                    border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
                    ..Default::default()
                }
            })
            .on_press_maybe(if enabled { Some(msg) } else { None })
            .padding([6, 14])
            .into()
        };

        let apply_btn = button(
            text("Apply & Save").style(move |_| iced::widget::text::Style { color: Some(c.bar_bg) })
        )
        .style(move |_, _| iced::widget::button::Style {
            background: Some(c.accent_fill),
            text_color: c.bar_bg,
            border: Border { radius: colors.radius.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::ApplyAndSave)
        .padding([6, 14]);

        let search_btn = button(
            text(if self.search_open { "× Close" } else { "Search" })
                .style(move |_| iced::widget::text::Style { color: Some(c.text) })
        )
        .style(move |_, status| iced::widget::button::Style {
            background: Some(Background::Color(match status {
                iced::widget::button::Status::Hovered => c.surface,
                _ => c.header_btn_bg,
            })),
            text_color: c.text,
            border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
            ..Default::default()
        })
        .on_press(Message::SearchToggle)
        .padding([6, 14]);

        let center: Element<Message> = if self.search_open {
            text_input("Search all settings…", &self.search_query)
                .id(search_input_id())
                .on_input(Message::SearchQueryChanged)
                .style(move |_, _| iced::widget::text_input::Style {
                    background: Background::Color(c.surface),
                    border: Border { color: c.accent, width: 1.0, radius: colors.radius.into() },
                    icon: c.dim_text,
                    placeholder: c.dim_text,
                    value: c.text,
                    selection: c.accent,
                })
                .width(320)
                .into()
        } else {
            text("OG Settings")
                .size(16)
                .style(move |_| iced::widget::text::Style { color: Some(c.text) })
                .into()
        };

        container(
            stack![
                container(search_btn)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Left)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding([0, 16]),
                container(center)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                container(
                    row![apply_btn, hdr_btn("X", Message::Close, true)]
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Center)
                .padding([0, 16]),
            ],
        )
        .height(48)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(c.bar_bg)),
            ..Default::default()
        })
        .into()
    }

    // ── Tab bar ───────────────────────────────────────────────────────────

    /// Left sidebar of tab buttons, in its own scrollable — a fixed-width
    /// column rather than the old horizontal row so adding more tabs never
    /// requires shrinking labels or wrapping; it just scrolls (the actual
    /// point of this layout switch — see PLAN.md-style reasoning: more
    /// settings tabs are coming, a top row runs out of horizontal room
    /// first).
    fn sidebar<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        let c = colors;
        // Each tab is a row of [indicator, button] so the indicator is a
        // left-edge accent bar instead of the old bottom-underline one —
        // same idea, rotated to match a vertical list.
        let tab_items: Vec<Element<Message>> = Tab::all()
            .iter()
            .map(|tab| {
                let active = tab == &self.current_tab;
                let t = tab.clone();
                let btn = button(
                    text(tab.label()).style(move |_| iced::widget::text::Style {
                        color: Some(if active { c.text } else { c.dim_text }),
                    })
                )
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(Background::Color(if active { c.surface } else { c.bar_bg })),
                    text_color: if active { c.text } else { c.dim_text },
                    border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
                    ..Default::default()
                })
                .on_press(Message::TabSelected(t))
                .padding([10, 16])
                .width(Length::Fill);

                let indicator = container(iced::widget::Space::new(2u16, Length::Fill))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(
                            if active { c.accent } else { Color::TRANSPARENT }
                        )),
                        ..Default::default()
                    })
                    .height(Length::Fill);

                row![indicator, btn].align_y(iced::Alignment::Center).into()
            })
            .collect();

        container(scrollable(column(tab_items).spacing(0)).height(Length::Fill))
            .style(move |_| container::Style {
                background: Some(Background::Color(c.bar_bg)),
                ..Default::default()
            })
            .width(Length::Fixed(180.0))
            .height(Length::Fill)
            .into()
    }

    // ── Search results ───────────────────────────────────────────────────
    //
    // Not every setting is inlined here — the plain toggles/sliders/pickers
    // (Power sleep settings, startup services, Theme colors/gaps/border/
    // taskbar/clock) get real, working controls right in the results. The
    // handful of stateful, canvas-driven screens (monitor arrange, taskbar
    // module arrange, hotkey capture, the embedded btop terminal) are too
    // involved to safely re-render out of context here, so those show up
    // as a labeled "Open" button that jumps to their tab instead.

    fn search_results_view<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        let c = colors;
        let query = self.search_query.to_lowercase();
        let matches = move |haystack: &str| haystack.to_lowercase().contains(&query);

        let card_style = move |_: &_| container::Style {
            background: Some(Background::Color(c.sec_bg)),
            border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
            ..Default::default()
        };
        let btn_style = move |_: &_, _| iced::widget::button::Style {
            background: Some(Background::Color(c.surface)),
            text_color: c.text,
            border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
            ..Default::default()
        };

        let badge = move |tab: &'static str| -> Element<'a, Message> {
            text(tab).size(10)
                .style(move |_| iced::widget::text::Style { color: Some(c.dim_text) })
                .into()
        };

        let spin = move |value: i64, minus: Message, plus: Message| -> Element<'a, Message> {
            row![
                button(text("-").style(move |_| iced::widget::text::Style { color: Some(c.text) }))
                    .style(btn_style).on_press(minus),
                container(text(value.to_string()).style(move |_| iced::widget::text::Style { color: Some(c.text) }))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(c.surface)),
                        border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
                        ..Default::default()
                    })
                    .padding([4, 12]),
                button(text("+").style(move |_| iced::widget::text::Style { color: Some(c.text) }))
                    .style(btn_style).on_press(plus),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
        };

        let jump_btn = move |tab: Tab| -> Element<'a, Message> {
            button(text("Open →").size(12).style(move |_| iced::widget::text::Style { color: Some(c.text) }))
                .style(btn_style)
                .on_press(Message::TabSelected(tab))
                .padding([6, 12])
                .into()
        };

        let mut rows: Vec<Element<Message>> = Vec::new();
        let mut push = |tab: &'static str, label: String, searchable: String, control: Element<'a, Message>| {
            if !matches(&format!("{tab} {label} {searchable}")) {
                return;
            }
            rows.push(
                container(
                    row![
                        column![badge(tab), text(label).style(move |_| iced::widget::text::Style { color: Some(c.text) })]
                            .spacing(2)
                            .width(Length::FillPortion(2)),
                        container(control).width(Length::FillPortion(3)).align_x(iced::alignment::Horizontal::Right),
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(12)
                )
                .style(card_style)
                .padding(14)
                .width(Length::Fill)
                .into()
            );
        };

        // ── Power ───────────────────────────────────────────────────────
        push("Power", "Monitor sleep".into(), "display idle screen off".into(),
            toggler(self.config.monitor_sleep.enabled).on_toggle(Message::MonitorSleepToggled).into());
        push("Power", "Monitor sleep minutes".into(), "display idle timeout".into(),
            spin(self.config.monitor_sleep.minutes as i64, Message::MonitorSleepMinus, Message::MonitorSleepPlus));
        push("Power", "System suspend".into(), "sleep power".into(),
            toggler(self.config.system_sleep.enabled).on_toggle(Message::SystemSleepToggled).into());
        push("Power", "System suspend minutes".into(), "sleep power timeout".into(),
            spin(self.config.system_sleep.minutes as i64, Message::SystemSleepMinus, Message::SystemSleepPlus));
        push("Power", "Screen lock".into(), "swaylock idle security".into(),
            toggler(self.config.screen_lock.enabled).on_toggle(Message::ScreenLockToggled).into());
        push("Power", "Screen lock minutes".into(), "swaylock idle timeout".into(),
            spin(self.config.screen_lock.minutes as i64, Message::ScreenLockMinus, Message::ScreenLockPlus));
        for (idx, e) in self.startup_execs.iter().enumerate() {
            push("Power", e.command.clone(), "startup exec sway command".into(),
                toggler(e.enabled).on_toggle(move |v| Message::StartupExecToggled(idx, v)).into());
        }
        for (idx, s) in self.systemd_services.iter().enumerate() {
            push("Power", s.unit.clone(), "startup service systemd user".into(),
                toggler(s.enabled).on_toggle(move |v| Message::SystemdServiceToggled(idx, v)).into());
        }
        for (idx, s) in self.system_services.iter().enumerate() {
            push("Power", s.unit.clone(), "startup service systemd system root".into(),
                toggler(s.enabled).on_toggle(move |v| Message::SystemServiceToggled(idx, v)).into());
        }

        // ── Display ─────────────────────────────────────────────────────
        push("Display", "Monitor resolution".into(), "display screen mode arrange".into(), jump_btn(Tab::Display));

        // ── Theme ───────────────────────────────────────────────────────
        // Color rows carry both their controls — the swatch (opens the color
        // picker modal) and the hex text field — same as the Theme tab itself,
        // so a search match doesn't strand you with only half the setting.
        let color_field = move |value: &'a str, key: &'static str| -> Element<'a, Message> {
            let swatch_color = tabs::theme::hex_to_color(value);
            let key_owned = key.to_string();
            let swatch_btn: Element<Message> = button(iced::widget::Space::new(26, 26))
                .style(move |_, status| {
                    let border_col = match status {
                        iced::widget::button::Status::Hovered => c.accent,
                        _ => Color::BLACK,
                    };
                    iced::widget::button::Style {
                        background: Some(Background::Color(swatch_color)),
                        border: Border { color: border_col, width: 1.0, radius: 13.0.into() },
                        ..Default::default()
                    }
                })
                .on_press(Message::ColorPickerOpen(key_owned))
                .into();
            let key_owned2 = key.to_string();
            let hex_input: Element<Message> = text_input("", value)
                .on_input(move |v| Message::ColorChanged(key_owned2.clone(), v))
                .style(move |_, _| iced::widget::text_input::Style {
                    background: Background::Color(c.surface),
                    border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
                    icon: c.dim_text, placeholder: c.dim_text, value: c.text, selection: c.accent,
                })
                .width(90)
                .into();
            row![swatch_btn, hex_input].spacing(8).align_y(iced::Alignment::Center).into()
        };
        push("Theme", "Bar background color".into(), "bar_bg hex".into(), color_field(&self.config.bar_bg, "bar_bg"));
        push("Theme", "Bar text color".into(), "bar_text hex".into(), color_field(&self.config.bar_text, "bar_text"));
        push("Theme", "Accent color".into(), "accent hex".into(), color_field(&self.config.accent, "accent"));
        push("Theme", "Secondary background color".into(), "sec_bg hex".into(), color_field(&self.config.sec_bg, "sec_bg"));
        push("Theme", "Inactive color".into(), "inactive hex".into(), color_field(&self.config.inactive_color, "inactive_color"));
        push("Theme", "Urgent color".into(), "urgent hex".into(), color_field(&self.config.urgent_color, "urgent_color"));

        let terminal_pick: Element<Message> = iced::widget::pick_list(
            self.available_terminals.as_slice(),
            if self.config.terminal.is_empty() { None } else { Some(self.config.terminal.clone()) },
            Message::TerminalChanged,
        )
        .style(move |_, _| iced::widget::pick_list::Style {
            background: Background::Color(c.surface),
            text_color: c.text,
            placeholder_color: c.dim_text,
            handle_color: c.dim_text,
            border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
        })
        .width(160)
        .into();
        push("Theme", "Default terminal".into(), "terminal emulator alacritty foot".into(), terminal_pick);

        push("Theme", "Window gaps (inner)".into(), "gaps spacing window".into(),
            spin(self.config.gaps_inner as i64, Message::GapsInnerMinus, Message::GapsInnerPlus));
        push("Theme", "Window gaps (outer)".into(), "gaps spacing screen edge".into(),
            spin(self.config.gaps_outer as i64, Message::GapsOuterMinus, Message::GapsOuterPlus));
        push("Theme", "Border width".into(), "window border pixels".into(),
            spin(self.config.border_width as i64, Message::BorderWidthMinus, Message::BorderWidthPlus));

        let edge_btn = move |label: &'static str, value: &'static str| -> Element<'a, Message> {
            let is_current = self.config.waybar_position == value;
            button(text(label).size(12).style(move |_| iced::widget::text::Style {
                color: Some(if is_current { c.bar_bg } else { c.text }),
            }))
            .style(move |_, _| iced::widget::button::Style {
                background: Some(Background::Color(if is_current { c.accent } else { c.surface })),
                text_color: if is_current { c.bar_bg } else { c.text },
                border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
                ..Default::default()
            })
            .on_press(Message::WaybarPositionChanged(value.to_string()))
            .padding([6, 10])
            .into()
        };
        push("Theme", "Taskbar position".into(), "waybar edge top bottom left right".into(),
            row![edge_btn("Top", "top"), edge_btn("Bottom", "bottom"), edge_btn("Left", "left"), edge_btn("Right", "right")]
                .spacing(6).into());

        push("Theme", "Taskbar thickness".into(), "waybar size pixels".into(),
            spin(self.config.waybar_thickness as i64, Message::WaybarThicknessMinus, Message::WaybarThicknessPlus));

        let tz_pick = move |value: &str, on_select: Box<dyn Fn(&'static str) -> Message + 'a>| -> Element<'a, Message> {
            let current = if value.is_empty() { "System Default" } else { value };
            iced::widget::pick_list(
                tabs::theme::TIMEZONES,
                tabs::theme::TIMEZONES.iter().find(|t| **t == current).copied(),
                on_select,
            )
            .style(move |_, _| iced::widget::pick_list::Style {
                background: Background::Color(c.surface),
                text_color: c.text,
                placeholder_color: c.dim_text,
                handle_color: c.dim_text,
                border: Border { color: c.border, width: 1.0, radius: colors.radius.into() },
            })
            .width(180)
            .into()
        };
        push("Theme", "Clock timezone".into(), "time zone taskbar".into(),
            tz_pick(&self.config.clock_timezone, Box::new(|v| Message::ClockTimezoneSelected(v.to_string()))));
        for (idx, cc) in self.config.extra_clocks.iter().enumerate() {
            let remove_btn: Element<Message> = button(text("×").style(move |_| iced::widget::text::Style { color: Some(c.text) }))
                .style(btn_style).on_press(Message::ClockRemove(idx)).padding([4, 10]).into();
            push("Theme", format!("Extra clock ({})", cc.timezone), "time zone taskbar".into(),
                row![
                    tz_pick(&cc.timezone, Box::new(move |v| Message::ClockExtraTimezoneSelected(idx, v.to_string()))),
                    remove_btn,
                ]
                .spacing(8).align_y(iced::Alignment::Center).into());
        }
        push("Theme", "Arrange taskbar modules".into(), "waybar left center right order layout".into(), jump_btn(Tab::Theme));

        // ── Hotkeys ─────────────────────────────────────────────────────
        for (key, cmd) in self.hotkey_bindings.iter() {
            push("Hotkeys", format!("{key} → {cmd}"), "hotkey keybinding bindsym".into(), jump_btn(Tab::Hotkeys));
        }
        for (name, val) in self.hotkey_variables.iter() {
            push("Hotkeys", format!("{name} = {val}"), "hotkey variable set mod".into(), jump_btn(Tab::Hotkeys));
        }

        if rows.is_empty() {
            rows.push(
                text(format!("No settings match \"{}\"", self.search_query))
                    .style(move |_| iced::widget::text::Style { color: Some(c.dim_text) })
                    .into()
            );
        }

        column(rows).spacing(10).padding(20).into()
    }

    // ── Content ───────────────────────────────────────────────────────────

    fn tab_content<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        match self.current_tab {
            Tab::Notifications => tabs::notifications::view(&self.config, colors, sway::is_notification_fx_running(), sway::is_notifications_enabled()),
            Tab::Power => tabs::power::view(&self.config, colors, &self.startup_execs, &self.systemd_services, &self.system_services, self.power_selected),
            Tab::Display => tabs::display::view(
                &self.config, colors, &self.monitors, self.arrange_mode,
                self.arrange_scale,
                &self.arrange_positions,
                self.arrange_dragging.as_ref().map(|(n, _, _)| n.as_str()),
            ),
            Tab::Network => tabs::network::view(
                colors,
                &self.wifi_status,
                &self.wifi_networks,
                &self.ethernet_interfaces,
                &self.bluetooth_adapter,
                &self.bluetooth_devices,
                self.network_scanning,
            ),
            Tab::Updates => tabs::updater::view(
                colors,
                &self.update_status,
                self.updates_checking,
                &self.updates_search,
                &self.updates_collapsed,
            ),
            Tab::Hotkeys => tabs::hotkeys::view(
                colors,
                &self.hotkey_variables,
                self.capturing_variable,
                &self.hotkey_bindings,
                self.capturing_hotkey,
            ),
            Tab::Theme => tabs::theme::view(
                &self.config,
                colors,
                self.color_picker_open.as_deref(),
                &self.theme_name,
                &self.imported_themes,
                &self.available_terminals,
                &self.available_cursor_themes,
                &self.available_browsers,
                &self.available_ai_clis,
                &self.available_wallpapers,
                self.has_backlight,
                self.brightness,
                self.module_arrange_mode,
                self.module_dragging.as_ref(),
            ),
            Tab::Bar => tabs::bar::view(&self.bar_config, colors),
            Tab::Search => tabs::search::view(&self.config, colors),
            Tab::History => tabs::history::view(&self.history, colors),
            Tab::SysMonitor => tabs::sysmon::view(colors, self.term_canvas.as_ref()),
        }
    }

    // ── Monitor snap logic ────────────────────────────────────────────────

    fn snap_to_edges(&mut self, dragged_name: String) {
        const SNAP_PX: f32 = 22.0; // canvas-pixel threshold
        let scale = self.arrange_scale;

        let dm = match self.monitors.iter().find(|m| m.name == dragged_name) {
            Some(m) => m.clone(),
            None => return,
        };
        let dw = (dm.width  as f32 * scale).max(60.0);
        let dh = (dm.height as f32 * scale).max(36.0);
        let (dx, dy) = *self.arrange_positions.get(&dragged_name)
            .unwrap_or(&(dm.x as f32 * scale + 10.0, dm.y as f32 * scale + 10.0));

        let mut best_x = dx;
        let mut best_y = dy;
        let mut min_dx = SNAP_PX;
        let mut min_dy = SNAP_PX;

        for m in &self.monitors {
            if m.name == dragged_name { continue; }
            let (ox, oy) = *self.arrange_positions.get(&m.name)
                .unwrap_or(&(m.x as f32 * scale + 10.0, m.y as f32 * scale + 10.0));
            let ow = (m.width  as f32 * scale).max(60.0);
            let oh = (m.height as f32 * scale).max(36.0);

            // X snaps: left-of-dragged to right-of-other, right-to-left, left-to-left, right-to-right
            let x_snaps = [
                (dx,        ox + ow),  // dragged.left  → other.right
                (dx + dw,   ox),       // dragged.right → other.left  (subtract dw)
                (dx,        ox),       // dragged.left  → other.left
                (dx + dw,   ox + ow),  // dragged.right → other.right (subtract dw)
            ];
            for (edge, target) in x_snaps {
                let d = (edge - target).abs();
                if d < min_dx {
                    min_dx = d;
                    // Compute where x should go so that edge aligns with target
                    best_x = if edge == dx { target } else { target - dw };
                }
            }

            // Y snaps
            let y_snaps = [
                (dy,        oy + oh),
                (dy + dh,   oy),
                (dy,        oy),
                (dy + dh,   oy + oh),
            ];
            for (edge, target) in y_snaps {
                let d = (edge - target).abs();
                if d < min_dy {
                    min_dy = d;
                    best_y = if edge == dy { target } else { target - dh };
                }
            }
        }

        self.arrange_positions.insert(dragged_name, (best_x.max(0.0), best_y.max(0.0)));
    }

    // ── Save-as-theme modal overlay ───────────────────────────────────────

    fn theme_save_modal<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        let name_input: Element<Message> = iced::widget::text_input("Theme name", &self.theme_save_name)
            .on_input(Message::ThemeSaveAsName)
            .on_submit(Message::ThemeSaveAsConfirm)
            .style(move |_, _| iced::widget::text_input::Style {
                background: Background::Color(colors.surface),
                border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                icon: colors.dim_text,
                placeholder: colors.dim_text,
                value: colors.text,
                selection: colors.accent,
            })
            .width(260)
            .into();

        let can_save = !self.theme_save_name.trim().is_empty();

        let card: Element<Message> = container(
            column![
                text("Save theme as").size(15)
                    .style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
                name_input,
                row![
                    iced::widget::horizontal_space(),
                    button(text("Cancel")
                        .style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
                        .style(move |_, _| iced::widget::button::Style {
                            background: Some(Background::Color(colors.surface)),
                            text_color: colors.text,
                            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                            ..Default::default()
                        })
                        .on_press(Message::ThemeSaveAsCancel)
                        .padding([8, 20]),
                    button(text("Save")
                        .style(move |_| iced::widget::text::Style { color: Some(colors.bar_bg) }))
                        .style(move |_, _| iced::widget::button::Style {
                            background: Some(Background::Color(colors.accent)),
                            text_color: colors.bar_bg,
                            border: Border { radius: colors.radius.into(), ..Default::default() },
                            ..Default::default()
                        })
                        .on_press_maybe(can_save.then_some(Message::ThemeSaveAsConfirm))
                        .padding([8, 20]),
                ]
                .spacing(10),
            ]
            .spacing(16)
            .padding(24),
        )
        .style(move |_| container::Style {
            background: Some(Background::Color(colors.sec_bg)),
            border: Border { color: colors.border, width: 1.0, radius: (colors.radius + 2.0).into() },
            ..Default::default()
        })
        .width(320)
        .into();

        container(card)
            .style(|_| container::Style {
                background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 })),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .into()
    }

    // ── Color picker modal overlay ────────────────────────────────────────

    fn color_picker_modal<'a>(&'a self, colors: AppColors) -> Element<'a, Message> {
        let key = match self.color_picker_open.as_deref() {
            Some(k) => k,
            None => return iced::widget::Space::new(0, 0).into(),
        };

        let preview_color = crate::color_wheel::hsv_to_rgb(self.color_picker_h, self.color_picker_s, self.color_picker_v);
        let hex_preview = format!(
            "#{:02x}{:02x}{:02x}",
            (preview_color.r * 255.0).round() as u8,
            (preview_color.g * 255.0).round() as u8,
            (preview_color.b * 255.0).round() as u8,
        );

        let brightness_row: Element<Message> = {
            use iced::widget::slider;
            row![
                container(
                    text("Brightness").style(move |_| iced::widget::text::Style { color: Some(colors.text) })
                ).width(70),
                slider(0.0f32..=1.0f32, self.color_picker_v, Message::ColorPickerValue).step(0.01).width(Length::Fill),
                container(
                    text(format!("{}%", (self.color_picker_v * 100.0).round() as u8))
                        .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
                ).width(44),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .into()
        };

        let picker_card: Element<Message> = container(
            column![
                row![
                    text(format!("Color — {}", key)).size(15)
                        .style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
                    iced::widget::horizontal_space(),
                    container(iced::widget::Space::new(64u16, 36u16))
                        .style(move |_| container::Style {
                            background: Some(Background::Color(preview_color)),
                            border: Border { color: colors.border, width: 1.0, radius: 18.0.into() },
                            ..Default::default()
                        }),
                    container(
                        text(hex_preview.clone()).size(13)
                            .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
                    ).padding([0, 10]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
                container(
                    crate::color_wheel::view(self.color_picker_h, self.color_picker_s, colors)
                )
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
                brightness_row,
                row![
                    iced::widget::horizontal_space(),
                    button(text("Cancel")
                        .style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
                        .style(move |_, _| iced::widget::button::Style {
                            background: Some(Background::Color(colors.surface)),
                            text_color: colors.text,
                            border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
                            ..Default::default()
                        })
                        .on_press(Message::ColorPickerClose)
                        .padding([8, 20]),
                    button(text("Apply")
                        .style(move |_| iced::widget::text::Style { color: Some(colors.bar_bg) }))
                        .style(move |_, _| iced::widget::button::Style {
                            background: Some(Background::Color(colors.accent)),
                            text_color: colors.bar_bg,
                            border: Border { radius: colors.radius.into(), ..Default::default() },
                            ..Default::default()
                        })
                        .on_press(Message::ColorPickerCommit)
                        .padding([8, 20]),
                ]
                .spacing(10),
            ]
            .spacing(16)
            .padding(24),
        )
        .style(move |_| container::Style {
            background: Some(Background::Color(colors.sec_bg)),
            border: Border { color: colors.accent, width: 2.0, radius: (colors.radius + 2.0).into() },
            ..Default::default()
        })
        .width(420)
        .into();

        // Dim backdrop + centered card
        container(picker_card)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 })),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .into()
    }
}

fn accent_line(color: Color) -> Element<'static, Message> {
    container(iced::widget::Space::new(Length::Fill, 2))
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(2)
        .into()
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn format_key(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> String {
    let mut parts: Vec<String> = Vec::new();
    if modifiers.logo() { parts.push("$mod".into()); }
    if modifiers.control() { parts.push("ctrl".into()); }
    if modifiers.alt() { parts.push("alt".into()); }
    if modifiers.shift() { parts.push("shift".into()); }

    match key {
        keyboard::Key::Named(named) => {
            use keyboard::key::Named;
            if !matches!(named, Named::Shift | Named::Control | Named::Alt | Named::Super | Named::Hyper | Named::Meta) {
                let name = format!("{named:?}").to_lowercase();
                parts.push(name);
            }
        }
        keyboard::Key::Character(c) => {
            parts.push(c.to_string());
        }
        _ => {}
    }
    parts.join("+")
}

// Maps a key press to an X11 modifier name (for variable capture)
fn format_modifier_key(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<String> {
    use keyboard::key::Named;
    if modifiers.logo() || matches!(key, keyboard::Key::Named(Named::Super)) {
        return Some("Mod4".into());
    }
    if modifiers.alt() || matches!(key, keyboard::Key::Named(Named::Alt)) {
        return Some("Mod1".into());
    }
    if modifiers.control() || matches!(key, keyboard::Key::Named(Named::Control)) {
        return Some("Control".into());
    }
    if modifiers.shift() || matches!(key, keyboard::Key::Named(Named::Shift)) {
        return Some("Shift".into());
    }
    None
}

fn dark_preset() -> Config {
    Config {
        bar_bg: "#1a1a2e".into(),
        sec_bg: "#2a2535".into(),
        bar_text: "#e0e0e0".into(),
        accent: "#ff7800".into(),
        inactive_color: "#3a3a4a".into(),
        urgent_color: "#ff4444".into(),
        ..Config::default()
    }
}

fn light_preset() -> Config {
    Config {
        bar_bg: "#f5f5f5".into(),
        sec_bg: "#e0e0e0".into(),
        bar_text: "#1a1a1a".into(),
        accent: "#0066cc".into(),
        inactive_color: "#aaaaaa".into(),
        urgent_color: "#cc0000".into(),
        ..Config::default()
    }
}

/// Terminal emulators found on PATH (plus the configured one, always first).
fn on_path_dirs(bin: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut paths: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(str::to_string)
        .collect();
    paths.push(format!("{}/.local/bin", home));
    paths.iter().any(|d| std::path::Path::new(d).join(bin).is_file())
}

fn detect_from_known(known: &[&str], current: &str) -> Vec<String> {
    let mut found: Vec<String> = known
        .iter()
        .filter(|t| on_path_dirs(t))
        .map(|t| t.to_string())
        .collect();
    if !current.is_empty() && !found.iter().any(|t| t == current) {
        found.insert(0, current.to_string());
    }
    found
}

fn detect_terminals(current: &str) -> Vec<String> {
    const KNOWN: &[&str] = &[
        "alacritty", "foot", "kitty", "wezterm", "konsole", "gnome-terminal",
        "xfce4-terminal", "tilix", "terminator", "urxvt", "st", "xterm",
    ];
    detect_from_known(KNOWN, current)
}

fn detect_browsers(current: &str) -> Vec<String> {
    const KNOWN: &[&str] = &[
        "brave", "brave-browser", "firefox", "firefox-esr", "chromium",
        "google-chrome-stable", "google-chrome", "vivaldi-stable",
        "epiphany", "qutebrowser", "opera",
    ];
    detect_from_known(KNOWN, current)
}

fn detect_ai_clis(current: &str) -> Vec<String> {
    const KNOWN: &[&str] = &[
        "claude", "gemini", "ollama", "aichat", "sgpt", "llm", "chatgpt",
    ];
    detect_from_known(KNOWN, current)
}

/// Translate an iced key press into terminal input bytes.
fn term_key_bytes(key: &keyboard::Key, text: Option<&str>) -> Option<Vec<u8>> {
    use keyboard::key::Named;
    if let keyboard::Key::Named(named) = key {
        let bytes: &[u8] = match named {
            Named::Enter => b"\r",
            Named::Escape => b"\x1b",
            Named::Backspace => b"\x7f",
            Named::Tab => b"\t",
            Named::Space => b" ",
            Named::ArrowUp => b"\x1b[A",
            Named::ArrowDown => b"\x1b[B",
            Named::ArrowRight => b"\x1b[C",
            Named::ArrowLeft => b"\x1b[D",
            Named::Home => b"\x1b[H",
            Named::End => b"\x1b[F",
            Named::PageUp => b"\x1b[5~",
            Named::PageDown => b"\x1b[6~",
            Named::Delete => b"\x1b[3~",
            Named::Insert => b"\x1b[2~",
            Named::F1 => b"\x1bOP",
            Named::F2 => b"\x1bOQ",
            Named::F3 => b"\x1bOR",
            Named::F4 => b"\x1bOS",
            Named::F5 => b"\x1b[15~",
            Named::F6 => b"\x1b[17~",
            Named::F7 => b"\x1b[18~",
            Named::F8 => b"\x1b[19~",
            Named::F9 => b"\x1b[20~",
            Named::F10 => b"\x1b[21~",
            Named::F11 => b"\x1b[23~",
            Named::F12 => b"\x1b[24~",
            _ => return None,
        };
        return Some(bytes.to_vec());
    }
    if let Some(t) = text {
        if !t.is_empty() {
            return Some(t.as_bytes().to_vec());
        }
    }
    if let keyboard::Key::Character(c) = key {
        return Some(c.as_bytes().to_vec());
    }
    None
}

/// Bundles every Network-tab data source into one background task —
/// `bluetoothctl info` runs once per device and `iwctl` calls aren't
/// instant, so this stays off the UI thread the same way the Power tab's
/// service lists do.
fn load_network_data() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                (
                    sway::get_wifi_status(),
                    sway::get_wifi_networks(),
                    sway::get_ethernet_interfaces(),
                    sway::get_bluetooth_adapter(),
                    sway::get_bluetooth_devices(),
                )
            })
            .await
            .unwrap_or_else(|_| (
                sway::WifiStatus::default(),
                Vec::new(),
                Vec::new(),
                sway::BluetoothAdapter::default(),
                Vec::new(),
            ))
        },
        |(wifi, networks, eth, bt_adapter, bt_devices)| {
            Message::NetworkDataLoaded(wifi, networks, eth, bt_adapter, bt_devices)
        },
    )
}

/// `checkupdates`/`yay -Qua`/`flatpak remote-ls` each shell out and can take
/// a second or two, so this runs off the UI thread like Power/Network do.
fn load_update_status() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(sway::get_update_status)
                .await
                .unwrap_or_default()
        },
        Message::UpdatesStatusLoaded,
    )
}

fn load_imported_themes() -> Vec<(String, Config)> {
    let home = std::env::var("HOME").unwrap_or_default();
    let themes_dir = format!("{}/themes", home);
    let mut themes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                        themes.push((name, cfg));
                    }
                }
            }
        }
    }
    themes
}

/// Splits a bindsym line into (flags + keys, command).
/// Flags like --locked / --to-code stay attached to the keys so the
/// line round-trips unchanged. The command is kept verbatim — sway
/// commands (`focus left`) and launches (`exec foo`) are both valid.
fn parse_bindsym(rest: &str) -> Option<(String, String)> {
    let mut rest = rest.trim_start();
    let mut keys_parts: Vec<&str> = Vec::new();
    while rest.starts_with("--") {
        let (flag, r) = rest.split_once(' ')?;
        keys_parts.push(flag);
        rest = r.trim_start();
    }
    let (keys, cmd) = rest.split_once(' ')?;
    keys_parts.push(keys);
    Some((keys_parts.join(" "), cmd.trim().to_string()))
}

/// Only bindings at brace depth 0 are editable — bindsym lines inside
/// `mode "..." { }` blocks must stay in their mode and are skipped.
fn load_sway_bindings() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{}/.config/sway/config", home);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    parse_sway_bindings(&content)
}

fn parse_sway_bindings(content: &str) -> Vec<(String, String)> {
    let mut bindings = Vec::new();
    let mut depth: i32 = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue; // comments must not affect brace depth
        }
        if trimmed.starts_with('}') {
            depth -= 1;
        }
        if depth == 0 {
            if let Some(rest) = trimmed.strip_prefix("bindsym ") {
                if let Some(b) = parse_bindsym(rest) {
                    bindings.push(b);
                }
            }
        }
        if trimmed.ends_with('{') {
            depth += 1;
        }
    }
    bindings
}

const VAR_START: &str = "# og-settings variables start";
const VAR_END: &str = "# og-settings variables end";
const BND_START: &str = "# og-settings bindings start";
const BND_END: &str = "# og-settings bindings end";

/// Rewrites the sway config with the edited variables and bindings.
///
/// Sway expands variables in the order the file is read, so a `set` must
/// appear before every line that uses it. Existing `set $x` lines are
/// therefore updated in place (keeping their position); brand-new variables
/// go into a managed block right after the last existing `set`. All
/// top-level bindsym lines are removed and rewritten in one managed block
/// at the end of the file, after every `set`. Bindings inside
/// `mode "..." { }` blocks are left untouched.
fn save_to_sway_config(variables: &[(String, String)], bindings: &[(String, String)]) {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{}/.config/sway/config", home);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::write(&path, rewrite_sway_config(&content, variables, bindings));
}

fn rewrite_sway_config(
    content: &str,
    variables: &[(String, String)],
    bindings: &[(String, String)],
) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut depth: i32 = 0;
    let mut in_managed = false;
    let mut updated: Vec<String> = Vec::new();
    let mut last_set_idx: Option<usize> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == VAR_START || trimmed == BND_START {
            in_managed = true;
            continue;
        }
        if trimmed == VAR_END || trimmed == BND_END {
            in_managed = false;
            continue;
        }
        if in_managed {
            continue;
        }
        if trimmed.starts_with('#') {
            // comments pass through untouched and must not affect brace depth
            out.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('}') {
            depth -= 1;
        }
        if depth == 0 {
            if trimmed.starts_with("bindsym ") {
                // top-level bindings are rewritten into the managed block below
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("set ") {
                if let Some((name, _)) = rest.split_once(' ') {
                    if name.starts_with('$') {
                        if updated.iter().any(|n| n == name) {
                            continue; // duplicate definition — drop it
                        }
                        updated.push(name.to_string());
                        match variables.iter().find(|(n, _)| n == name) {
                            Some((_, val)) => {
                                out.push(format!("set {} {}", name, val));
                                last_set_idx = Some(out.len() - 1);
                            }
                            None => {} // variable removed in the UI — drop the line
                        }
                        continue;
                    }
                }
            }
        }
        if trimmed.ends_with('{') {
            depth += 1;
        }
        out.push(line.to_string());
    }

    // Variables that don't exist in the file yet: insert after the last
    // existing `set` so they still precede any possible use.
    let new_vars: Vec<String> = variables
        .iter()
        .filter(|(n, _)| !n.is_empty() && !updated.iter().any(|u| u == n))
        .map(|(n, v)| format!("set {} {}", n, v))
        .collect();
    if !new_vars.is_empty() {
        let idx = last_set_idx.map(|i| i + 1).unwrap_or(0);
        let mut block = vec![VAR_START.to_string()];
        block.extend(new_vars);
        block.push(VAR_END.to_string());
        for (off, l) in block.into_iter().enumerate() {
            out.insert(idx + off, l);
        }
    }

    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.push(String::new());
    out.push(BND_START.to_string());
    for (k, cmd) in bindings.iter().filter(|(k, c)| !k.is_empty() && !c.is_empty()) {
        out.push(format!("bindsym {} {}", k, cmd));
    }
    out.push(BND_END.to_string());

    out.join("\n") + "\n"
}

/// Top-level `set $name value` lines; first definition of a name wins,
/// duplicates are ignored (save() also drops them from the file).
fn load_sway_variables() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{}/.config/sway/config", home);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    parse_sway_variables(&content)
}

fn parse_sway_variables(content: &str) -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = Vec::new();
    let mut depth: i32 = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue; // comments must not affect brace depth
        }
        if trimmed.starts_with('}') {
            depth -= 1;
        }
        if depth == 0 {
            if let Some(rest) = trimmed.strip_prefix("set ") {
                if let Some((name, val)) = rest.split_once(' ') {
                    if name.starts_with('$') && !vars.iter().any(|(n, _)| n == name) {
                        vars.push((name.to_string(), val.trim().to_string()));
                    }
                }
            }
        }
        if trimmed.ends_with('{') {
            depth += 1;
        }
    }
    vars
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod config_rewrite_tests {
    use super::*;

    const SAMPLE: &str = "\
set $mod Mod4
set $left h
floating_modifier $mod normal
bindsym $mod+Return exec alacritty
bindsym $mod+$left focus left
bindsym --locked XF86AudioMute exec pactl set-sink-mute @DEFAULT_SINK@ toggle
mode \"resize\" {
    bindsym h resize shrink width 10px
    bindsym Return mode \"default\"
}
bindsym $mod+r mode \"resize\"
";

    fn vars(content: &str) -> Vec<(String, String)> {
        parse_sway_variables(content)
    }

    fn binds(content: &str) -> Vec<(String, String)> {
        parse_sway_bindings(content)
    }

    #[test]
    fn parse_keeps_flags_and_exec() {
        assert_eq!(
            parse_bindsym("--locked XF86AudioMute exec pactl foo"),
            Some(("--locked XF86AudioMute".into(), "exec pactl foo".into()))
        );
        assert_eq!(
            parse_bindsym("$mod+h focus left"),
            Some(("$mod+h".into(), "focus left".into()))
        );
    }

    #[test]
    fn mode_block_bindings_are_not_loaded() {
        let b = binds(SAMPLE);
        assert!(b.iter().all(|(k, _)| k != "h" && k != "Return"));
        assert_eq!(b.len(), 4);
    }

    #[test]
    fn roundtrip_is_stable_and_valid() {
        let v = vars(SAMPLE);
        let b = binds(SAMPLE);
        let once = rewrite_sway_config(SAMPLE, &v, &b);

        // no duplicate bindings, no exec added to sway commands
        assert_eq!(once.matches("bindsym $mod+Return").count(), 1);
        assert!(once.contains("bindsym $mod+$left focus left"));
        assert!(!once.contains("exec focus"));
        // mode block internals untouched
        assert!(once.contains("    bindsym h resize shrink width 10px"));
        assert!(once.contains("    bindsym Return mode \"default\""));
        // set lines keep their position before first use
        let set_pos = once.find("set $mod").unwrap();
        let use_pos = once.find("floating_modifier").unwrap();
        assert!(set_pos < use_pos);

        // saving again without edits must be a no-op
        let twice = rewrite_sway_config(&once, &vars(&once), &binds(&once));
        assert_eq!(once, twice);
    }

    #[test]
    fn changing_mod_updates_set_in_place() {
        let mut v = vars(SAMPLE);
        v.iter_mut().find(|(n, _)| n == "$mod").unwrap().1 = "Mod1".into();
        let out = rewrite_sway_config(SAMPLE, &v, &binds(SAMPLE));
        assert!(out.contains("set $mod Mod1"));
        assert!(!out.contains("set $mod Mod4"));
        assert_eq!(out.matches("set $mod").count(), 1);
    }

    #[test]
    fn commented_braces_do_not_break_depth_tracking() {
        // A commented-out block whose closing brace is also commented used to
        // leave the parser stuck at depth > 0, hiding every binding below it.
        let sample = "\
set $mod Mod4
#   input type:touchpad {
#       tap enabled
#   }
bindsym $mod+Return exec alacritty
bindsym $mod+q kill
";
        let b = binds(sample);
        assert_eq!(b.len(), 2);
        assert_eq!(vars(sample).len(), 1);

        // rewrite must keep the commented block verbatim and stay stable
        let once = rewrite_sway_config(sample, &vars(sample), &b);
        assert!(once.contains("#   input type:touchpad {"));
        assert!(once.contains("#   }"));
        let twice = rewrite_sway_config(&once, &vars(&once), &binds(&once));
        assert_eq!(once, twice);
    }

    #[test]
    fn new_variable_lands_before_bindings() {
        let mut v = vars(SAMPLE);
        v.push(("$term".into(), "foot".into()));
        let out = rewrite_sway_config(SAMPLE, &v, &binds(SAMPLE));
        let set_pos = out.find("set $term foot").unwrap();
        let bnd_pos = out.find(BND_START).unwrap();
        assert!(set_pos < bnd_pos);
    }

    #[test]
    fn hsv_roundtrip_matches_known_colors() {
        use crate::color_wheel::{hsv_to_rgb, rgb_to_hsv};
        let cases: &[(&str, u8, u8, u8)] = &[
            ("orange accent", 0xff, 0x78, 0x00),
            ("dark bg", 0x1a, 0x1a, 0x2e),
            ("white", 0xff, 0xff, 0xff),
            ("black", 0x00, 0x00, 0x00),
            ("gray", 0x80, 0x80, 0x80),
            ("pure red", 0xff, 0x00, 0x00),
            ("pure green", 0x00, 0xff, 0x00),
            ("pure blue", 0x00, 0x00, 0xff),
        ];
        for (name, r, g, b) in cases {
            let original = iced::Color::from_rgb8(*r, *g, *b);
            let (h, s, v) = rgb_to_hsv(original);
            let back = hsv_to_rgb(h, s, v);
            let round = |x: f32| (x * 255.0).round() as u8;
            assert_eq!((round(back.r), round(back.g), round(back.b)), (*r, *g, *b), "mismatch for {name}");
        }
    }
}
