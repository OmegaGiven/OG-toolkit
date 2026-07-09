use iced::widget::{column, container, row, text};
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

    /// tz abbreviation / date / time, as separate segments so the caller
    /// can lay them out in a row (top/bottom bar) or a column (left/right
    /// bar) instead of a single string hardcoded to one direction.
    fn segments(&self) -> [String; 3] {
        let fmt = if self.hour12 { "%Z\n%m/%d\n%I:%M %p" } else { "%Z\n%m/%d\n%H:%M" };

        let explicit_tz = if self.timezone.is_empty() { None } else { self.timezone.parse::<chrono_tz::Tz>().ok() };

        let formatted = match explicit_tz.or_else(detect_system_tz) {
            Some(tz) => chrono::Utc::now().with_timezone(&tz).format(fmt).to_string(),
            None => chrono::Local::now().format(fmt).to_string(),
        };
        let mut parts = formatted.splitn(3, '\n');
        [
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
        ]
    }
}

impl Module for Clock {
    fn cell_length(&self, _size: u32, _orientation: Orientation) -> Length {
        // A fixed size×size square (the trait default) fits the old
        // 3-line stacked layout but clips a horizontal row of 3 segments
        // down to almost nothing — this needs to shrink to its own
        // content instead, same as cpu/memory's cell.
        Length::Shrink
    }

    fn view(&self, colors: AppColors, _size: u32, orientation: Orientation) -> Element<'_, Message> {
        let text_style = move |_: &_| text::Style { color: Some(colors.text) };
        let segments = self.segments();
        let widgets = segments.into_iter().map(|s| text(s).size(11).style(text_style).into());

        let content: Element<'_, Message> = match orientation {
            Orientation::Horizontal => row(widgets).spacing(6).align_y(iced::Alignment::Center).into(),
            Orientation::Vertical => column(widgets).align_x(iced::Alignment::Center).into(),
        };

        container(content).padding(4).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
    }
}
