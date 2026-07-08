//! One trait every bar module implements — adding a new module later means
//! implementing this trait, not touching bar layout code (PLAN.md section 5).

use iced::{Element, Length, Subscription};
use og_config::Edge;
use og_theme::AppColors;

use crate::message::Message;

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
