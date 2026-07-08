mod bar;
mod message;
mod module;
mod modules;
mod popup;
mod power;

use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::LayerShellSettings;

use bar::Bar;
use og_config::{BarConfig, Edge};

fn main() -> iced_layershell::Result {
    let bar_cfg = BarConfig::load();
    let thickness = bar_cfg.thickness;

    let (size, anchor) = match bar_cfg.position {
        Edge::Top => (Some((0, thickness)), Anchor::Top | Anchor::Left | Anchor::Right),
        Edge::Bottom => (Some((0, thickness)), Anchor::Bottom | Anchor::Left | Anchor::Right),
        Edge::Left => (Some((thickness, 0)), Anchor::Left | Anchor::Top | Anchor::Bottom),
        Edge::Right => (Some((thickness, 0)), Anchor::Right | Anchor::Top | Anchor::Bottom),
    };

    daemon("og-bar", bar::update, bar::view, bar::remove_id)
        .layer_settings(LayerShellSettings {
            size,
            anchor,
            exclusive_zone: thickness as i32,
            ..Default::default()
        })
        .style(bar::style)
        .subscription(bar::subscription)
        .run_with(Bar::new)
}
