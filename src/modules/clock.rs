use iced::widget::{container, text};
use iced::{Element, Length, Subscription};

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

pub struct Clock {
    /// Empty means local time. Anything else is parsed as an IANA tz
    /// name (e.g. "America/Chicago") on every tick — invalid names just
    /// fall back to local time rather than showing garbage.
    timezone: String,
}

impl Clock {
    pub fn new(timezone: String) -> Self {
        Self { timezone }
    }

    fn label(&self) -> String {
        if self.timezone.is_empty() {
            return chrono::Local::now().format("%H:%M\n%m/%d").to_string();
        }
        match self.timezone.parse::<chrono_tz::Tz>() {
            Ok(tz) => chrono::Utc::now().with_timezone(&tz).format("%H:%M\n%m/%d").to_string(),
            Err(_) => chrono::Local::now().format("%H:%M\n%m/%d").to_string(),
        }
    }
}

impl Module for Clock {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        container(
            text(self.label())
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
