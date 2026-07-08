// Step 1 skeleton: single horizontal top bar, layer-shell surface up, empty.
// Proves iced_layershell + og-theme integration before any modules are built.

use iced::widget::container;
use iced::{Element, Length, Task};

use iced_layershell::build_pattern::application;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;

use og_config::{BarConfig, Config};
use og_theme::AppColors;

fn main() -> iced_layershell::Result {
    let bar_cfg = BarConfig::load();
    let thickness = bar_cfg.thickness;

    application("og-bar", update, view)
        .layer_settings(LayerShellSettings {
            size: Some((0, thickness)),
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            exclusive_zone: thickness as i32,
            ..Default::default()
        })
        .style(style)
        .run_with(Bar::new)
}

struct Bar {
    colors: AppColors,
}

impl Bar {
    fn new() -> (Self, Task<Message>) {
        let colors = AppColors::from_config(&Config::load(), "og-bar");
        (Self { colors }, Task::none())
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {}

fn update(_state: &mut Bar, message: Message) -> Task<Message> {
    match message {
        _ => Task::none(),
    }
}

fn view(state: &Bar) -> Element<'_, Message> {
    container(iced::widget::Space::new(Length::Fill, Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(state.colors.bg_fill),
            ..Default::default()
        })
        .into()
}

fn style(state: &Bar, _theme: &iced::Theme) -> iced_layershell::Appearance {
    iced_layershell::Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: state.colors.text,
    }
}
