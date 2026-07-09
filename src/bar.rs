use iced::widget::{column, container, row};
use iced::{Element, Length, Subscription, Task};

use iced_layershell::actions::{IcedNewMenuSettings, MenuDirection};

use og_config::{BarConfig, Config, Edge, ModuleConfig, ModuleKind};
use og_theme::AppColors;

use crate::message::Message;
use crate::module::{Module, Orientation};
use crate::modules::bluetooth::Bluetooth;
use crate::modules::clock::Clock;
use crate::modules::cpu::Cpu;
use crate::modules::launcher::Launcher;
use crate::modules::memory::Memory;
use crate::modules::network::Network;
use crate::modules::pulseaudio::Pulseaudio;
use crate::modules::workspaces::Workspaces;
use crate::popup::{PopupKind, PopupState};
use crate::power::PowerButton;
use crate::settings::{Section, SettingsButton};

fn build_modules(list: &[ModuleConfig], icon_rewrite: &[og_config::IconRewriteRule]) -> Vec<Box<dyn Module>> {
    list.iter()
        .filter(|m| m.enabled)
        .filter_map(|m| -> Option<Box<dyn Module>> {
            match &m.kind {
                ModuleKind::Workspaces => Some(Box::new(Workspaces::new(icon_rewrite.to_vec()))),
                ModuleKind::Clock { timezone } => Some(Box::new(Clock::new(timezone.clone()))),
                ModuleKind::Launcher { icon, command, .. } => {
                    Some(Box::new(Launcher::new(icon.clone(), command.clone())))
                }
                ModuleKind::Power => Some(Box::new(PowerButton)),
                ModuleKind::Settings => Some(Box::new(SettingsButton)),
                ModuleKind::Cpu => Some(Box::new(Cpu::new())),
                ModuleKind::Memory => Some(Box::new(Memory::new())),
                ModuleKind::Network => Some(Box::new(Network::new())),
                ModuleKind::Bluetooth => Some(Box::new(Bluetooth::new())),
                ModuleKind::Pulseaudio => Some(Box::new(Pulseaudio::new())),
                // Tray: step 8 spike (real unknown, per PLAN.md section 9/13).
                ModuleKind::Tray => None,
            }
        })
        .collect()
}

fn section_list(bar_cfg: &mut BarConfig, section: Section) -> &mut Vec<ModuleConfig> {
    match section {
        Section::Start => &mut bar_cfg.modules_start,
        Section::Center => &mut bar_cfg.modules_center,
        Section::End => &mut bar_cfg.modules_end,
    }
}

pub struct Bar {
    colors: AppColors,
    bar_cfg: BarConfig,
    start: Vec<Box<dyn Module>>,
    center: Vec<Box<dyn Module>>,
    end: Vec<Box<dyn Module>>,
    popup: Option<PopupState>,
}

impl Bar {
    pub fn new() -> (Self, Task<Message>) {
        let bar_cfg = BarConfig::load();
        let colors = AppColors::from_config(&Config::load(), "og-bar");
        let bar = Self {
            start: build_modules(&bar_cfg.modules_start, &bar_cfg.icon_rewrite),
            center: build_modules(&bar_cfg.modules_center, &bar_cfg.icon_rewrite),
            end: build_modules(&bar_cfg.modules_end, &bar_cfg.icon_rewrite),
            colors,
            bar_cfg,
            popup: None,
        };
        (bar, Task::none())
    }

    fn all_modules_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Module>> {
        self.start.iter_mut().chain(self.center.iter_mut()).chain(self.end.iter_mut())
    }

    fn all_modules(&self) -> impl Iterator<Item = &Box<dyn Module>> {
        self.start.iter().chain(self.center.iter()).chain(self.end.iter())
    }
}

pub fn remove_id(bar: &mut Bar, id: iced::window::Id) {
    if bar.popup.as_ref().is_some_and(|p| p.id == id) {
        bar.popup = None;
    }
}

pub fn update(bar: &mut Bar, message: Message) -> Task<Message> {
    match &message {
        Message::FocusWorkspace(name) => {
            let name = name.clone();
            return Task::future(async move {
                let _ = tokio::process::Command::new("swaymsg")
                    .arg(format!("workspace {name}"))
                    .output()
                    .await;
                Message::Tick
            });
        }
        Message::Launch(cmd) => {
            let cmd = cmd.clone();
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("setsid {cmd} >/dev/null 2>&1 &"))
                .spawn();
        }
        Message::OpenPowerMenu => {
            let id = iced::window::Id::unique();
            bar.popup = Some(PopupState { id, kind: PopupKind::Power });
            // Top/vertical bars: menu should open below the click point.
            // Bottom bar: open above it instead, so it doesn't run off-screen.
            let direction = match bar.bar_cfg.position {
                Edge::Bottom => MenuDirection::Up,
                Edge::Top | Edge::Left | Edge::Right => MenuDirection::Down,
            };
            return Task::done(Message::NewMenu {
                settings: IcedNewMenuSettings { size: (140, 190), direction },
                id,
            });
        }
        Message::PowerAction(action) => {
            action.run();
            if let Some(popup) = bar.popup.take() {
                return Task::done(Message::RemoveWindow(popup.id));
            }
        }
        Message::ClosePopup => {
            if let Some(popup) = bar.popup.take() {
                return Task::done(Message::RemoveWindow(popup.id));
            }
        }
        Message::ClickOnWindow(window) => {
            if let Some(popup) = &bar.popup {
                if popup.id != *window {
                    let id = popup.id;
                    bar.popup = None;
                    return Task::done(Message::RemoveWindow(id));
                }
            }
        }
        Message::OpenSettingsPopup => {
            let id = iced::window::Id::unique();
            bar.popup = Some(PopupState { id, kind: PopupKind::Settings });
            let direction = match bar.bar_cfg.position {
                Edge::Bottom => MenuDirection::Up,
                Edge::Top | Edge::Left | Edge::Right => MenuDirection::Down,
            };
            return Task::done(Message::NewMenu {
                settings: IcedNewMenuSettings { size: (260, 420), direction },
                id,
            });
        }
        // Edge/thickness need a real wlr layer-shell relayout this process
        // can't do to its own already-mapped surface. Edge is a discrete
        // button click, not a drag — nothing is lost by applying it (i.e.
        // respawning) the instant it's clicked, and requiring a separate
        // "Apply" click after was a real discoverability gap (a user
        // clicking Left/Right/Bottom reasonably expects the bar to just
        // move). Thickness is a slider — respawning on every tick of a
        // drag would spam processes, so that one still stages and needs
        // the explicit Apply button.
        Message::SetEdge(edge) => {
            bar.bar_cfg.position = *edge;
            let _ = bar.bar_cfg.save();
            return Task::done(Message::ApplyRelayout);
        }
        Message::SetThickness(v) => {
            bar.bar_cfg.thickness = *v;
            let _ = bar.bar_cfg.save();
        }
        // item_size/spacing/padding are read straight out of bar_cfg by
        // bar_view() every frame, so these apply live with no extra work.
        Message::SetItemSize(v) => {
            bar.bar_cfg.item_size = *v;
            let _ = bar.bar_cfg.save();
        }
        Message::SetSpacing(v) => {
            bar.bar_cfg.spacing = *v;
            let _ = bar.bar_cfg.save();
        }
        Message::SetPadding(v) => {
            bar.bar_cfg.padding = *v;
            let _ = bar.bar_cfg.save();
        }
        Message::ToggleModule(section, index) => {
            if let Some(m) = section_list(&mut bar.bar_cfg, *section).get_mut(*index) {
                m.enabled = !m.enabled;
            }
            let _ = bar.bar_cfg.save();
            bar.start = build_modules(&bar.bar_cfg.modules_start, &bar.bar_cfg.icon_rewrite);
            bar.center = build_modules(&bar.bar_cfg.modules_center, &bar.bar_cfg.icon_rewrite);
            bar.end = build_modules(&bar.bar_cfg.modules_end, &bar.bar_cfg.icon_rewrite);
        }
        Message::SetClockTimezone(section, index, tz) => {
            if let Some(m) = section_list(&mut bar.bar_cfg, *section).get_mut(*index) {
                if let ModuleKind::Clock { timezone } = &mut m.kind {
                    *timezone = tz.clone();
                }
            }
            let _ = bar.bar_cfg.save();
            bar.start = build_modules(&bar.bar_cfg.modules_start, &bar.bar_cfg.icon_rewrite);
            bar.center = build_modules(&bar.bar_cfg.modules_center, &bar.bar_cfg.icon_rewrite);
            bar.end = build_modules(&bar.bar_cfg.modules_end, &bar.bar_cfg.icon_rewrite);
        }
        Message::ApplyRelayout => {
            let _ = bar.bar_cfg.save();
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe).spawn();
            }
            std::process::exit(0);
        }
        _ => {}
    }

    for module in bar.all_modules_mut() {
        module.update(&message);
    }
    Task::none()
}

pub fn view(bar: &Bar, id: iced::window::Id) -> Element<'_, Message> {
    if let Some(popup) = &bar.popup {
        if popup.id == id {
            return match popup.kind {
                PopupKind::Power => crate::power::popup_view(bar.colors),
                PopupKind::Settings => crate::settings::popup_view(bar.colors, &bar.bar_cfg),
            };
        }
    }
    bar_view(bar)
}

fn bar_view(bar: &Bar) -> Element<'_, Message> {
    let size = bar.bar_cfg.item_size;
    let thickness = bar.bar_cfg.thickness as u16;
    let spacing = bar.bar_cfg.spacing as u16;
    let orientation = Orientation::from(bar.bar_cfg.position);

    // Main axis = the direction the bar runs (width for top/bottom, height
    // for left/right). Cross axis is always the bar's fixed thickness.
    fn cell<'a>(m: &'a Box<dyn Module>, bar: &'a Bar, size: u32, orientation: Orientation, thickness: u16) -> Element<'a, Message> {
        let main_len = m.cell_length(size, orientation);
        let content = m.view(bar.colors, size, orientation);
        let c = container(content).center_x(Length::Fill).center_y(Length::Fill);
        match orientation {
            Orientation::Horizontal => c.width(main_len).height(thickness),
            Orientation::Vertical => c.width(thickness).height(main_len),
        }
        .into()
    }

    fn section<'a>(
        modules: &'a [Box<dyn Module>],
        bar: &'a Bar,
        size: u32,
        orientation: Orientation,
        thickness: u16,
        spacing: u16,
    ) -> Element<'a, Message> {
        let cells = modules.iter().map(|m| cell(m, bar, size, orientation, thickness));
        match orientation {
            Orientation::Horizontal => row(cells).spacing(spacing).into(),
            Orientation::Vertical => column(cells).spacing(spacing).into(),
        }
    }

    let content: Element<'_, Message> = match orientation {
        Orientation::Horizontal => row![
            container(section(&bar.start, bar, size, orientation, thickness, spacing))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Left),
            container(section(&bar.center, bar, size, orientation, thickness, spacing))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
            container(section(&bar.end, bar, size, orientation, thickness, spacing))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .padding(bar.bar_cfg.padding as u16)
        .align_y(iced::Alignment::Center)
        .into(),
        Orientation::Vertical => column![
            container(section(&bar.start, bar, size, orientation, thickness, spacing))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Top),
            container(section(&bar.center, bar, size, orientation, thickness, spacing))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center),
            container(section(&bar.end, bar, size, orientation, thickness, spacing))
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom),
        ]
        .padding(bar.bar_cfg.padding as u16)
        .align_x(iced::Alignment::Center)
        .into(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bar.colors.bg_fill),
            ..Default::default()
        })
        .into()
}

pub fn subscription(bar: &Bar) -> Subscription<Message> {
    let escape_closes_popup = iced::event::listen_with(|event, _status, _id| {
        if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) = event
        {
            Some(Message::ClosePopup)
        } else {
            None
        }
    });

    // A real "click anywhere on the desktop closes the popup" needs the
    // compositor to grab pointer input for the popup surface (xdg_popup's
    // own `.grab()`) and deliver a `popup_done` event on outside click —
    // iced_layershell 0.13.7 never calls `.grab()` and its xdg_popup
    // dispatch doesn't even handle `PopupDone` if the compositor sent one
    // anyway. That's an upstream gap, not fixable from here. What this
    // *can* do: notice a click landing back on the bar's own surface
    // (a different window from the popup) while a popup is open, and
    // treat that as "dismiss" — covers the common case of clicking the
    // bar again, though not clicking some unrelated app window elsewhere.
    // `listen_with` only takes a non-capturing fn pointer, so this always
    // reports the click; `update()` (which has `bar.popup`) decides
    // whether it actually means "close".
    // Status::Ignored means no widget (button, etc) claimed the click —
    // critically, this excludes the click that just opened a popup via
    // its own on_press, which iced reports as Captured. Without this
    // check, clicking the gear/power icon would open a popup and this
    // same listener would immediately close it again in the same frame.
    let click_reports_window = iced::event::listen_with(|event, status, event_window| {
        if status != iced::event::Status::Ignored {
            return None;
        }
        if let iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) = event {
            Some(Message::ClickOnWindow(event_window))
        } else {
            None
        }
    });

    Subscription::batch(
        bar.all_modules()
            .map(|m| m.subscription())
            .chain([escape_closes_popup, click_reports_window]),
    )
}

pub fn style(bar: &Bar, _theme: &iced::Theme) -> iced_layershell::Appearance {
    iced_layershell::Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: bar.colors.text,
    }
}
