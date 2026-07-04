use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Background, Border, Color, Element, Length};

use crate::app::{AppColors, Message, ModuleDrag};
use crate::config::Config;
use crate::tabs::taskbar_arrange;

pub const TIMEZONES: &[&str] = &[
    "System Default",
    "UTC",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Anchorage",
    "America/Sao_Paulo",
    "America/Mexico_City",
    "America/Toronto",
    "America/Vancouver",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Madrid",
    "Europe/Rome",
    "Europe/Moscow",
    "Europe/Istanbul",
    "Africa/Cairo",
    "Africa/Johannesburg",
    "Africa/Lagos",
    "Asia/Dubai",
    "Asia/Karachi",
    "Asia/Kolkata",
    "Asia/Dhaka",
    "Asia/Bangkok",
    "Asia/Jakarta",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Singapore",
    "Australia/Sydney",
    "Australia/Perth",
    "Australia/Melbourne",
    "Pacific/Auckland",
    "Pacific/Honolulu",
];

pub fn hex_to_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return Color::BLACK;
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128);
    Color::from_rgb8(r, g, b)
}

pub fn view<'a>(
    config: &'a Config,
    colors: AppColors,
    picker_key: Option<&'a str>,  // kept for swatch highlight only
    theme_name: &'a str,
    imported_themes: &'a [(String, Config)],
    available_terminals: &'a [String],
    available_cursor_themes: &'a [String],
    module_arrange_mode: bool,
    module_dragging: Option<&'a ModuleDrag>,
) -> Element<'a, Message> {
    let card_style = move |_: &_| container::Style {
        background: Some(Background::Color(colors.sec_bg)),
        border: Border { color: colors.border, width: 1.0, radius: 6.0.into() },
        ..Default::default()
    };

    let btn_style = move |_theme: &iced::Theme, _status: iced::widget::button::Status| iced::widget::button::Style {
        background: Some(Background::Color(colors.surface)),
        text_color: colors.text,
        border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
        ..Default::default()
    };

    // ── Preset / theme buttons ────────────────────────────────────────────
    let make_preset_btn = move |name: &'static str| -> Element<'a, Message> {
        let is_active = theme_name == name;
        let bg = if is_active { colors.accent } else { colors.surface };
        let fg = if is_active { colors.bar_bg } else { colors.text };
        button(
            text(format!("{} {}", if is_active { "●" } else { "○" }, name))
                .size(13)
                .style(move |_| iced::widget::text::Style { color: Some(fg) })
        )
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            text_color: fg,
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::ThemePreset(name.to_string()))
        .padding([6, 14])
        .into()
    };

    let mut preset_row_items: Vec<Element<Message>> = vec![
        make_preset_btn("Dark"),
        make_preset_btn("Light"),
    ];

    // Custom indicator
    if !matches!(theme_name, "Dark" | "Light") && imported_themes.iter().all(|(n, _)| n != theme_name) {
        preset_row_items.push(
            button(
                text(format!("● {}", theme_name)).size(13)
                    .style(move |_| iced::widget::text::Style { color: Some(colors.bar_bg) })
            )
            .style(move |_, _| iced::widget::button::Style {
                background: Some(Background::Color(colors.accent)),
                text_color: colors.bar_bg,
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([6, 14])
            .into()
        );
    }

    // Imported themes
    for (i, (name, _)) in imported_themes.iter().enumerate() {
        let is_active = theme_name == name.as_str();
        let bg = if is_active { colors.accent } else { colors.surface };
        let fg = if is_active { colors.bar_bg } else { colors.text };
        let n = name.clone();
        preset_row_items.push(
            button(
                text(format!("{} {}", if is_active { "●" } else { "○" }, n))
                    .size(13)
                    .style(move |_| iced::widget::text::Style { color: Some(fg) })
            )
            .style(move |_, _| iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(Message::ImportedThemeSelect(i))
            .padding([6, 14])
            .into()
        );
    }

    preset_row_items.push(iced::widget::horizontal_space().into());

    preset_row_items.push(
        button(
            text("Save As…").size(13)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
        )
        .style(btn_style)
        .on_press(Message::ThemeSaveAsOpen)
        .padding([6, 14])
        .into()
    );

    preset_row_items.push(
        button(
            text("Import").size(13)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
        )
        .style(btn_style)
        .on_press(Message::ThemeImport)
        .padding([6, 14])
        .into()
    );

    preset_row_items.push(
        button(
            text("Export").size(13)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
        )
        .style(btn_style)
        .on_press(Message::ThemeExport)
        .padding([6, 14])
        .into()
    );

    let presets_card = container(
        column![
            text("Themes").size(15)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
            row(preset_row_items).spacing(8).align_y(iced::Alignment::Center),
        ]
        .spacing(12)
        .padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    // ── Color palette ─────────────────────────────────────────────────────
    let palette_entries: &[(&str, &str, &str, &str)] = &[
        ("bar_bg",         &config.bar_bg,         "Background",           "Waybar bg · Window base · Panel bg"),
        ("sec_bg",         &config.sec_bg,         "Secondary background", "Panel sections · Focused window title bg"),
        ("bar_text",       &config.bar_text,       "Text",                 "Waybar text · Window title text"),
        ("accent",         &config.accent,         "Accent",               "Waybar active · Focused border & indicator"),
        ("inactive_color", &config.inactive_color, "Inactive",             "Inactive & unfocused window borders"),
        ("urgent_color",   &config.urgent_color,   "Urgent",               "Urgent window border & highlight"),
    ];

    let color_rows: Vec<Element<Message>> = palette_entries.iter().map(|(key, hex, label, hint)| {
        let swatch_color = hex_to_color(hex);
        let key_str = key.to_string();
        let hex_owned = hex.to_string();
        let is_open = picker_key == Some(key);

        let swatch_btn: Element<Message> = button(
            iced::widget::Space::new(28, 28)
        )
        .style(move |_, status| {
            let border_col = if is_open {
                colors.accent
            } else {
                match status {
                    iced::widget::button::Status::Hovered => colors.accent,
                    _ => Color::BLACK,
                }
            };
            iced::widget::button::Style {
                background: Some(Background::Color(swatch_color)),
                border: Border { color: border_col, width: if is_open { 2.0 } else { 1.0 }, radius: 14.0.into() },
                ..Default::default()
            }
        })
        .on_press(Message::ColorPickerOpen(key_str.clone()))
        .into();

        let hex_input: Element<Message> = text_input("", &hex_owned)
            .on_input(move |v| Message::ColorChanged(key_str.clone(), v))
            .style(move |_, _| iced::widget::text_input::Style {
                background: Background::Color(colors.surface),
                border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
                icon: colors.dim_text,
                placeholder: colors.dim_text,
                value: colors.text,
                selection: colors.accent,
            })
            .width(110)
            .into();

        row![
            swatch_btn,
            container(
                text(*label).style(move |_| iced::widget::text::Style { color: Some(colors.text) })
            ).width(160),
            hex_input,
            text(*hint).size(12)
                .style(move |_| iced::widget::text::Style { color: Some(colors.dim_text) }),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }).collect();

    let mut colors_col: Vec<Element<Message>> = vec![
        text("System Colors").size(15)
            .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
            .into(),
    ];
    colors_col.extend(color_rows);

    let colors_card = container(
        column(colors_col).spacing(12).padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    // ── Terminal setting ──────────────────────────────────────────────────
    let terminal_card = container(
        column![
            text("Default Terminal").size(15)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
            row![
                text("Terminal emulator")
                    .style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
                iced::widget::horizontal_space(),
                pick_list(
                    available_terminals,
                    if config.terminal.is_empty() { None } else { Some(config.terminal.clone()) },
                    Message::TerminalChanged,
                )
                .style(move |_, _| iced::widget::pick_list::Style {
                    background: Background::Color(colors.surface),
                    text_color: colors.text,
                    placeholder_color: colors.dim_text,
                    handle_color: colors.dim_text,
                    border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
                })
                .width(200),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(12),
        ]
        .spacing(16)
        .padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    // ── Gaps & borders ────────────────────────────────────────────────────
    let label = move |t: &'a str| -> Element<'a, Message> {
        text(t).style(move |_| iced::widget::text::Style { color: Some(colors.text) }).into()
    };
    let spin = move |value: i32, minus: Message, plus: Message| -> Element<'a, Message> {
        row![
            button(text("-").style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
                .style(btn_style).on_press(minus),
            container(text(value.to_string()).style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
                .style(move |_| container::Style {
                    background: Some(Background::Color(colors.surface)),
                    border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
                    ..Default::default()
                })
                .padding([4, 12]),
            button(text("+").style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
                .style(btn_style).on_press(plus),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    };

    let window_card = container(
        column![
            text("Window Settings").size(15).style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
            row![label("Inner gaps (between windows)"), iced::widget::horizontal_space(),
                spin(config.gaps_inner, Message::GapsInnerMinus, Message::GapsInnerPlus)]
                .align_y(iced::Alignment::Center).spacing(12),
            row![label("Outer gaps (screen edge)"), iced::widget::horizontal_space(),
                spin(config.gaps_outer, Message::GapsOuterMinus, Message::GapsOuterPlus)]
                .align_y(iced::Alignment::Center).spacing(12),
            row![label("Border width (pixels)"), iced::widget::horizontal_space(),
                spin(config.border_width, Message::BorderWidthMinus, Message::BorderWidthPlus)]
                .align_y(iced::Alignment::Center).spacing(12),
        ]
        .spacing(16).padding(20),
    )
    .style(card_style).width(Length::Fill);

    // ── Taskbar (waybar edge + thickness) ──────────────────────────────────
    let edge_button = move |label: &'static str, value: &'static str| -> Element<'a, Message> {
        let is_current = config.waybar_position == value;
        button(
            text(label).size(12).style(move |_| iced::widget::text::Style {
                color: Some(if is_current { colors.bar_bg } else { colors.text }),
            })
        )
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(if is_current { colors.accent } else { colors.surface })),
            text_color: if is_current { colors.bar_bg } else { colors.text },
            border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
            ..Default::default()
        })
        .on_press(Message::WaybarPositionChanged(value.to_string()))
        .padding([6, 14])
        .into()
    };

    let tz_pick = move |value: &str, on_select: Box<dyn Fn(&'static str) -> Message + 'a>| -> Element<'a, Message> {
        let current = if value.is_empty() { "System Default" } else { value };
        pick_list(TIMEZONES, TIMEZONES.iter().find(|t| **t == current).copied(), on_select)
            .style(move |_, _| iced::widget::pick_list::Style {
                background: Background::Color(colors.surface),
                text_color: colors.text,
                placeholder_color: colors.dim_text,
                handle_color: colors.dim_text,
                border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
            })
            .width(220)
            .into()
    };

    let remove_btn = move |msg: Message| -> Element<'a, Message> {
        button(text("×").style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
            .style(btn_style)
            .on_press(msg)
            .padding([4, 10])
            .into()
    };

    let mut extra_clock_rows: Vec<Element<Message>> = config.extra_clocks.iter().enumerate().map(|(idx, cc)| {
        row![
            label("Extra clock timezone"), iced::widget::horizontal_space(),
            tz_pick(&cc.timezone, Box::new(move |v| Message::ClockExtraTimezoneSelected(idx, v.to_string()))),
            remove_btn(Message::ClockRemove(idx)),
        ]
        .align_y(iced::Alignment::Center).spacing(12)
        .into()
    }).collect();

    let add_clock_btn = button(
        text("+ Add Clock").size(12).style(move |_| iced::widget::text::Style { color: Some(colors.text) })
    )
    .style(btn_style)
    .on_press(Message::ClockAdd)
    .padding([6, 14]);

    let mut taskbar_col: Vec<Element<Message>> = vec![
        text("Taskbar").size(15)
            .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
            .into(),
        row![label("Screen edge"), iced::widget::horizontal_space(),
            row![
                edge_button("Top", "top"),
                edge_button("Bottom", "bottom"),
                edge_button("Left", "left"),
                edge_button("Right", "right"),
            ]
            .spacing(8),
        ]
        .align_y(iced::Alignment::Center).spacing(12)
        .into(),
        row![label("Thickness (pixels)"), iced::widget::horizontal_space(),
            spin(config.waybar_thickness, Message::WaybarThicknessMinus, Message::WaybarThicknessPlus)]
            .align_y(iced::Alignment::Center).spacing(12)
            .into(),
        row![label("Clock timezone"), iced::widget::horizontal_space(),
            tz_pick(&config.clock_timezone, Box::new(|v| Message::ClockTimezoneSelected(v.to_string())))]
            .align_y(iced::Alignment::Center).spacing(12)
            .into(),
    ];
    taskbar_col.append(&mut extra_clock_rows);
    taskbar_col.push(row![iced::widget::horizontal_space(), add_clock_btn].into());

    let arrange_btn = if module_arrange_mode {
        button(
            text("Exit Arrange").size(12)
                .style(move |_| iced::widget::text::Style { color: Some(colors.bar_bg) })
        )
        .style(move |_, _| iced::widget::button::Style {
            background: Some(Background::Color(colors.accent)),
            text_color: colors.bar_bg,
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
    } else {
        button(
            text("Arrange Modules").size(12)
                .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
        )
        .style(btn_style)
    }
    .on_press(Message::ModuleArrangeModeToggle)
    .padding([6, 14]);

    taskbar_col.push(
        row![
            label("Module positions (left / center / right)"),
            iced::widget::horizontal_space(),
            arrange_btn,
        ]
        .align_y(iced::Alignment::Center).spacing(12)
        .into()
    );
    if module_arrange_mode {
        taskbar_col.push(taskbar_arrange::view(config, colors, module_dragging).into());
        taskbar_col.push(
            text("Drag a module chip to reorder it or move it between Left / Center / Right.")
                .size(11)
                .style(move |_| iced::widget::text::Style { color: Some(colors.dim_text) })
                .into()
        );
    }

    let taskbar_card = container(
        column(taskbar_col)
        .spacing(16).padding(20),
    )
    .style(card_style).width(Length::Fill);

    // ── Mouse & cursor ──────────────────────────────────────────────────────
    let sensitivity_row = {
        use iced::widget::slider;
        row![
            label("Pointer sensitivity"),
            slider(-1.0f32..=1.0f32, config.mouse_sensitivity, Message::MouseSensitivityChanged)
                .step(0.05)
                .width(220),
            container(
                text(format!("{:+.2}", config.mouse_sensitivity))
                    .style(move |_| iced::widget::text::Style { color: Some(colors.text) })
            ).width(50),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(12)
    };

    let cursor_theme_options: Vec<String> = if available_cursor_themes.is_empty() {
        vec![config.cursor_theme.clone()]
    } else {
        available_cursor_themes.to_vec()
    };
    let cursor_theme_row = row![
        label("Cursor icon theme"),
        iced::widget::horizontal_space(),
        pick_list(
            cursor_theme_options,
            if config.cursor_theme.is_empty() { None } else { Some(config.cursor_theme.clone()) },
            Message::CursorThemeChanged,
        )
        .style(move |_, _| iced::widget::pick_list::Style {
            background: Background::Color(colors.surface),
            text_color: colors.text,
            placeholder_color: colors.dim_text,
            handle_color: colors.dim_text,
            border: Border { color: colors.border, width: 1.0, radius: 4.0.into() },
        })
        .width(200),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(12);

    let mouse_card = container(
        column![
            text("Mouse & Cursor").size(15).style(move |_| iced::widget::text::Style { color: Some(colors.text) }),
            sensitivity_row,
            cursor_theme_row,
            row![label("Cursor size (pixels)"), iced::widget::horizontal_space(),
                spin(config.cursor_size, Message::CursorSizeMinus, Message::CursorSizePlus)]
                .align_y(iced::Alignment::Center).spacing(12),
        ]
        .spacing(16).padding(20),
    )
    .style(card_style).width(Length::Fill);

    column![presets_card, colors_card, window_card, taskbar_card, mouse_card, terminal_card]
        .spacing(16)
        .padding(20)
        .into()
}
