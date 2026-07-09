//! One trait every bar module implements — adding a new module later means
//! implementing this trait, not touching bar layout code (PLAN.md section 5).

use iced::widget::{column, row, text};
use iced::{Color, Element, Length, Subscription};
use og_config::Edge;
use og_theme::AppColors;

use crate::message::Message;

/// Shared "short label + value" layout for cpu/memory-style modules.
/// Horizontal bars lay the two side by side (room to breathe along the
/// bar's own running direction); vertical bars stack label above value
/// (that's the direction with room there instead). Either way this is
/// sized to its own content (`Length::Shrink` via the caller's
/// `cell_length` override) rather than forced into the fixed square every
/// other module uses — that fixed square is what forced padding to 0 just
/// to keep a value like "100%" from clipping.
pub fn label_value<'a>(label: &'a str, value: String, color: Color, orientation: Orientation) -> Element<'a, Message> {
    let label_widget = text(label).size(10).style(move |_| text::Style { color: Some(Color { a: 0.65, ..color }) });
    let value_widget = text(value).size(12).style(move |_| text::Style { color: Some(color) });
    match orientation {
        Orientation::Horizontal => row![label_widget, value_widget].spacing(4).align_y(iced::Alignment::Center).into(),
        Orientation::Vertical => column![label_widget, value_widget].align_x(iced::Alignment::Center).into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl From<Edge> for Orientation {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::Top | Edge::Bottom => Orientation::Horizontal,
            Edge::Left | Edge::Right => Orientation::Vertical,
        }
    }
}

pub trait Module {
    fn view(&self, colors: AppColors, size: u32, orientation: Orientation) -> Element<'_, Message>;

    /// Most modules render into a fixed `size` along the bar's main axis
    /// (section 6) — width for a horizontal bar, height for a vertical
    /// one. Edge-spanning modules like workspaces override this to
    /// `Shrink` regardless of orientation.
    fn cell_length(&self, size: u32, _orientation: Orientation) -> Length {
        Length::Fixed(size as f32)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn update(&mut self, _message: &Message) {}
}
