use iced::widget::{container, text};
use iced::{Element, Length, Subscription};

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

/// Non-empty `timezone` (multi-tz clocks) isn't wired yet — needs a tz
/// database crate (`chrono-tz`) not currently a dependency anywhere in
/// OG-toolkit. Local time only for now.
pub struct Clock {
    #[allow(dead_code)]
    timezone: String,
}

impl Clock {
    pub fn new(timezone: String) -> Self {
        Self { timezone }
    }
}

impl Module for Clock {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        let now = chrono::Local::now();
        let label = now.format("%H:%M\n%m/%d").to_string();
        container(
            text(label)
                .size(13)
                .align_x(iced::alignment::Horizontal::Center)
                .style(move |_| text::Style { color: Some(colors.text) }),
        )
        .width(size as u16)
        .height(size as u16)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
    }
}
