//! One trait every bar module implements — adding a new module later means
//! implementing this trait, not touching bar layout code (PLAN.md section 5).

use iced::{Element, Length, Subscription};
use og_theme::AppColors;

use crate::message::Message;

pub trait Module {
    fn view(&self, colors: AppColors, size: u32) -> Element<'_, Message>;

    /// Most modules render into a fixed `size x size` cell (section 6).
    /// Edge-spanning modules like workspaces override this to `Shrink`.
    fn cell_width(&self, size: u32) -> Length {
        Length::Fixed(size as f32)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn update(&mut self, _message: &Message) {}
}
