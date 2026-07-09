mod bar;
mod icon_font;
mod message;
mod module;
mod modules;
mod popup;
mod power;
mod settings;
mod window_menu;

use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};

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

    let app = daemon("og-bar", bar::update, bar::view, bar::remove_id);
    let app = match icon_font::load_font_bytes() {
        Some(bytes) => app.font(bytes),
        None => app,
    };

    app.layer_settings(LayerShellSettings {
            size,
            anchor,
            exclusive_zone: thickness as i32,
            // One surface per connected output (waybar's own default
            // behavior) instead of just whichever output is "active" —
            // layershellev handles this itself, no manual output-tracking
            // needed on our side (see PLAN.md section 10 / step 9 spike).
            start_mode: StartMode::AllScreens,
            ..Default::default()
        })
        .style(bar::style)
        .subscription(bar::subscription)
        .run_with(Bar::new)
}
