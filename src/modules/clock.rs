use iced::widget::{container, text};
use iced::{Element, Length, Subscription};

use crate::message::Message;
use crate::module::{Module, Orientation};
use og_theme::AppColors;

/// `/etc/localtime` is a symlink into the zoneinfo database on every distro
/// that matters here — reading it back out gets us the IANA name for
/// "local time" so it can go through chrono-tz too and get a real `%Z`
/// abbreviation (EST/CST/etc), instead of chrono::Local's offset-only one.
fn detect_system_tz() -> Option<chrono_tz::Tz> {
    let link = std::fs::read_link("/etc/localtime").ok()?;
    let path = link.to_string_lossy();
    let name = path.split("zoneinfo/").nth(1)?;
    name.parse().ok()
}

pub struct Clock {
    /// Empty means local time (auto-detected via /etc/localtime for a
    /// proper %Z abbreviation). Anything else is parsed as an IANA tz name
    /// (e.g. "America/Chicago") on every tick — invalid names fall back to
    /// local time rather than showing garbage.
    timezone: String,
    hour12: bool,
}

impl Clock {
    pub fn new(timezone: String, hour12: bool) -> Self {
        Self { timezone, hour12 }
    }

    fn label(&self) -> String {
        let fmt = if self.hour12 { "%Z\n%m/%d\n%I:%M %p" } else { "%Z\n%m/%d\n%H:%M" };

        let explicit_tz = if self.timezone.is_empty() { None } else { self.timezone.parse::<chrono_tz::Tz>().ok() };

        match explicit_tz.or_else(detect_system_tz) {
            Some(tz) => chrono::Utc::now().with_timezone(&tz).format(fmt).to_string(),
            None => chrono::Local::now().format(fmt).to_string(),
        }
    }
}

impl Module for Clock {
    fn view(&self, colors: AppColors, size: u32, _orientation: Orientation) -> Element<'_, Message> {
        container(
            text(self.label())
                .size(11)
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
