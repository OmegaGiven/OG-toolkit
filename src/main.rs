mod bar;
mod message;
mod module;
mod modules;

use iced_layershell::build_pattern::application;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::LayerShellSettings;

use bar::Bar;
use og_config::BarConfig;

fn main() -> iced_layershell::Result {
    let bar_cfg = BarConfig::load();
    let thickness = bar_cfg.thickness;

    application("og-bar", bar::update, bar::view)
        .layer_settings(LayerShellSettings {
            size: Some((0, thickness)),
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            exclusive_zone: thickness as i32,
            ..Default::default()
        })
        .style(bar::style)
        .subscription(bar::subscription)
        .run_with(Bar::new)
}
