use iced::widget::{column, container, row};
use iced::{Element, Length, Subscription, Task};

use og_config::{BarConfig, Config, ModuleConfig, ModuleKind};
use og_theme::AppColors;

use crate::message::Message;
use crate::module::{Module, Orientation};
use crate::modules::clock::Clock;
use crate::modules::launcher::Launcher;
use crate::modules::workspaces::Workspaces;

fn build_modules(list: &[ModuleConfig]) -> Vec<Box<dyn Module>> {
    list.iter()
        .filter(|m| m.enabled)
        .filter_map(|m| -> Option<Box<dyn Module>> {
            match &m.kind {
                ModuleKind::Workspaces => Some(Box::new(Workspaces::new())),
                ModuleKind::Clock { timezone } => Some(Box::new(Clock::new(timezone.clone()))),
                ModuleKind::Launcher { icon, command, .. } => {
                    Some(Box::new(Launcher::new(icon.clone(), command.clone())))
                }
                // Cpu/Memory/Tray/Bluetooth/Network/Pulseaudio/Settings: later build steps.
                _ => None,
            }
        })
        .collect()
}

pub struct Bar {
    colors: AppColors,
    bar_cfg: BarConfig,
    start: Vec<Box<dyn Module>>,
    center: Vec<Box<dyn Module>>,
    end: Vec<Box<dyn Module>>,
}

impl Bar {
    pub fn new() -> (Self, Task<Message>) {
        let bar_cfg = BarConfig::load();
        let colors = AppColors::from_config(&Config::load(), "og-bar");
        let bar = Self {
            start: build_modules(&bar_cfg.modules_start),
            center: build_modules(&bar_cfg.modules_center),
            end: build_modules(&bar_cfg.modules_end),
            colors,
            bar_cfg,
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
        _ => {}
    }

    for module in bar.all_modules_mut() {
        module.update(&message);
    }
    Task::none()
}

pub fn view(bar: &Bar) -> Element<'_, Message> {
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
    Subscription::batch(bar.all_modules().map(|m| m.subscription()))
}

pub fn style(bar: &Bar, _theme: &iced::Theme) -> iced_layershell::Appearance {
    iced_layershell::Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: bar.colors.text,
    }
}
