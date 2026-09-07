use iced::{
    widget::{button, container, row, text, text_input},
    Background, Border, Color, Element, Length,
};
use crate::app::{Message, ViewMode, ACCENT, SEC_BG, TEXT, SURFACE};

const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

pub fn view<'a>(
    can_back: bool,
    can_forward: bool,
    path_text: &'a str,
    editing: bool,
    search_query: &'a str,
    view_mode: &ViewMode,
    show_hidden: bool,
) -> Element<'a, Message> {
    let nav_btn = |label: &'static str, msg: Message, enabled: bool| {
        let b = button(text(label).size(14))
            .style(move |_, status| button::Style {
                background: Some(Background::Color(if enabled {
                    match status {
                        button::Status::Hovered => SURFACE(),
                        _ => SEC_BG(),
                    }
                } else {
                    Color { r: 0.15, g: 0.13, b: 0.18, a: 1.0 }
                })),
                text_color: if enabled { TEXT() } else { Color { r: 0.4, g: 0.4, b: 0.45, a: 1.0 } },
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([4, 10]);
        if enabled { b.on_press(msg) } else { b }
    };

    let icon_btn = |icon: &'static str, msg: Message| {
        button(text(icon).font(ICON_FONT).size(14))
            .style(|_, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Hovered => SURFACE(),
                    _ => SEC_BG(),
                })),
                text_color: TEXT(),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([4, 10])
            .on_press(msg)
    };

    // Icon + a plain "+" as two separate texts (not one glyph run) so the
    // plus renders normally instead of getting cramped against the Nerd
    // Font icon.
    let icon_plus_btn = |icon: &'static str, msg: Message| {
        button(
            row![
                text(icon).font(ICON_FONT).size(14),
                text("+").size(13),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center)
        )
        .style(|_, status| button::Style {
            background: Some(Background::Color(match status {
                button::Status::Hovered => SURFACE(),
                _ => SEC_BG(),
            })),
            text_color: TEXT(),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([4, 10])
        .on_press(msg)
    };

    let path_bar: Element<Message> = if editing {
        text_input("Path...", path_text)
            .on_input(Message::PathBarEdit)
            .on_submit(Message::PathBarSubmit)
            .size(13)
            .style(|_, _| text_input::Style {
                background: Background::Color(SEC_BG()),
                border: Border {
                    color: ACCENT(),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: TEXT(),
                placeholder: Color { r: 0.5, g: 0.5, b: 0.55, a: 1.0 },
                value: TEXT(),
                selection: ACCENT(),
            })
            .into()
    } else {
        button(text(path_text).size(13))
            .on_press(Message::PathBarEdit(path_text.to_string()))
            .style(|_, _| button::Style {
                background: Some(Background::Color(SEC_BG())),
                text_color: TEXT(),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
    };

    let search = text_input("Search...", search_query)
        .on_input(Message::SearchChanged)
        .on_submit(Message::SearchSubmit)
        .size(13)
        .width(180)
        .style(|_, _| text_input::Style {
            background: Background::Color(SEC_BG()),
            border: Border { color: Color { r: 0.3, g: 0.3, b: 0.4, a: 1.0 }, width: 1.0, radius: 4.0.into() },
            icon: TEXT(),
            placeholder: Color { r: 0.5, g: 0.5, b: 0.55, a: 1.0 },
            value: TEXT(),
            selection: ACCENT(),
        });

    // Icon + a plain word, same "two separate texts" trick as icon_plus_btn —
    // some symbol glyphs (e.g. the old "⊞") aren't in the default UI font at
    // all and rendered as a tofu box, so view-mode/hidden-files icons are
    // always drawn in the Nerd Font instead of relying on generic Unicode.
    let icon_word_btn = |icon: &'static str, label: &'static str, msg: Message| {
        button(
            row![
                text(icon).font(ICON_FONT).size(14),
                text(label).size(13),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
        )
        .style(|_, status| button::Style {
            background: Some(Background::Color(match status {
                button::Status::Hovered => SURFACE(),
                _ => SEC_BG(),
            })),
            text_color: TEXT(),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([4, 10])
        .on_press(msg)
    };

    let (view_icon, view_label) = match view_mode {
        ViewMode::Grid => ("\u{f00b}", "List"),
        ViewMode::List => ("\u{f00a}", "Grid"),
    };
    let (hidden_icon, hidden_label) = if show_hidden {
        ("\u{f070}", "Hide")
    } else {
        ("\u{f06e}", "Show")
    };

    let toolbar_row = row![
        nav_btn("←", Message::NavigateBack, can_back),
        nav_btn("→", Message::NavigateForward, can_forward),
        nav_btn("↑", Message::NavigateUp, true),
        icon_btn("\u{f015}", Message::NavigateHome),
        container(path_bar).width(Length::Fill).padding([0, 6]),
        search,
        icon_word_btn(view_icon, view_label, Message::ViewModeToggle),
        icon_word_btn(hidden_icon, hidden_label, Message::ShowHiddenToggle),
        icon_btn("\u{f021}", Message::Refresh),
        icon_plus_btn("\u{f07b}", Message::NewFolder),
        icon_plus_btn("\u{f0f6}", Message::NewFile),
    ]
    .spacing(4)
    .padding([6, 8])
    .align_y(iced::Alignment::Center);

    container(toolbar_row)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SEC_BG())),
            border: Border {
                color: Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 },
                width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
