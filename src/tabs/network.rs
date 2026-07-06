use iced::widget::{button, column, container, row, scrollable, text, toggler};
use iced::{Background, Border, Color, Element, Length};

use crate::app::{AppColors, Message};
use crate::sway::{BluetoothAdapter, BluetoothDevice, EthernetInterface, WifiNetwork, WifiStatus};

pub fn view<'a>(
    colors: AppColors,
    wifi_status: &'a WifiStatus,
    wifi_networks: &'a [WifiNetwork],
    ethernet_interfaces: &'a [EthernetInterface],
    bluetooth_adapter: &'a BluetoothAdapter,
    bluetooth_devices: &'a [BluetoothDevice],
    scanning: bool,
) -> Element<'a, Message> {
    let card_style = move |_: &_| container::Style {
        background: Some(Background::Color(colors.sec_bg)),
        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    };
    let btn_style = move |_: &_, _| iced::widget::button::Style {
        background: Some(Background::Color(colors.surface)),
        text_color: colors.text,
        border: Border { color: colors.border, width: 1.0, radius: colors.radius.into() },
        ..Default::default()
    };
    let accent_btn_style = move |_: &_, _| iced::widget::button::Style {
        background: Some(Background::Color(colors.accent)),
        text_color: colors.bar_bg,
        border: Border { radius: colors.radius.into(), ..Default::default() },
        ..Default::default()
    };
    let danger_btn_style = move |_: &_, _| iced::widget::button::Style {
        background: Some(Background::Color(Color { r: 0.6, g: 0.1, b: 0.1, a: 1.0 })),
        text_color: Color::WHITE,
        border: Border { radius: colors.radius.into(), ..Default::default() },
        ..Default::default()
    };

    let title = move |t: &'a str| -> Element<'a, Message> {
        text(t).size(15).style(move |_| iced::widget::text::Style { color: Some(colors.text) }).into()
    };
    let label = move |t: String| -> Element<'a, Message> {
        text(t).style(move |_| iced::widget::text::Style { color: Some(colors.text) }).into()
    };
    let dim = move |t: String| -> Element<'a, Message> {
        text(t).size(12).style(move |_| iced::widget::text::Style { color: Some(colors.dim_text) }).into()
    };
    #[derive(Clone, Copy)]
    enum ActionKind { Plain, Accent, Danger }
    let action_btn = move |label: &'static str, msg: Message, kind: ActionKind| -> Element<'a, Message> {
        let text_color = match kind {
            ActionKind::Plain => colors.text,
            ActionKind::Accent => colors.bar_bg,
            ActionKind::Danger => Color::WHITE,
        };
        let btn = button(text(label).size(12).style(move |_| iced::widget::text::Style { color: Some(text_color) }))
            .on_press(msg)
            .padding([4, 10]);
        match kind {
            ActionKind::Plain => btn.style(btn_style),
            ActionKind::Accent => btn.style(accent_btn_style),
            ActionKind::Danger => btn.style(danger_btn_style),
        }
        .into()
    };

    // ── Wi-Fi ────────────────────────────────────────────────────────────
    let wifi_status_row: Element<Message> = if wifi_status.connected_ssid.is_some() {
        row![
            dim(format!(
                "Connected to {} {}",
                wifi_status.connected_ssid.as_deref().unwrap_or(""),
                wifi_status.ip.as_deref().map(|ip| format!("· {ip}")).unwrap_or_default(),
            )),
            iced::widget::horizontal_space(),
            action_btn("Disconnect", Message::WifiDisconnect, ActionKind::Plain),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        dim("Not connected".to_string())
    };

    let scan_label: &'static str = if scanning { "Scanning…" } else { "Scan" };
    let wifi_scan_btn = button(text(scan_label).size(12).style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
        .style(btn_style)
        .on_press_maybe(if scanning || !wifi_status.powered { None } else { Some(Message::WifiScanStart) })
        .padding([6, 14]);

    let mut wifi_rows: Vec<Element<Message>> = Vec::new();
    if wifi_status.powered {
        if wifi_networks.is_empty() {
            wifi_rows.push(dim("No networks found. Try Scan.".to_string()));
        } else {
            for net in wifi_networks {
                let is_connected = wifi_status.connected_ssid.as_deref() == Some(net.ssid.as_str());
                let bars = "●".repeat(net.signal_bars as usize) + &"○".repeat(4usize.saturating_sub(net.signal_bars as usize));
                let lock = if net.security.is_empty() || net.security.eq_ignore_ascii_case("open") { "" } else { " 🔒" };

                let action: Element<Message> = if is_connected {
                    action_btn("Disconnect", Message::WifiDisconnect, ActionKind::Plain)
                } else {
                    action_btn("Connect", Message::WifiConnect(net.ssid.clone()), ActionKind::Accent)
                };
                let forget: Element<Message> = if net.known {
                    action_btn("Forget", Message::WifiForget(net.ssid.clone()), ActionKind::Danger)
                } else {
                    iced::widget::Space::new(0, 0).into()
                };

                wifi_rows.push(
                    row![
                        text(bars).size(11).style(move |_| iced::widget::text::Style { color: Some(colors.accent) }).width(50),
                        column![
                            label(format!("{}{}", net.ssid, lock)),
                            dim(if is_connected { "Connected".to_string() } else { net.security.clone() }),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        forget,
                        action,
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10)
                    .into()
                );
            }
        }
    }

    let wifi_card = container(
        column(
            std::iter::once(
                row![
                    title("Wi-Fi"),
                    iced::widget::horizontal_space(),
                    wifi_scan_btn,
                    toggler(wifi_status.powered).on_toggle(Message::WifiPowerToggled),
                ]
                .align_y(iced::Alignment::Center)
                .spacing(12)
                .into()
            )
            .chain(std::iter::once(wifi_status_row))
            .chain(wifi_rows)
            .collect::<Vec<_>>()
        )
        .spacing(12)
        .padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    // ── Ethernet ─────────────────────────────────────────────────────────
    let eth_rows: Vec<Element<Message>> = if ethernet_interfaces.is_empty() {
        vec![dim("No wired interfaces found.".to_string())]
    } else {
        ethernet_interfaces.iter().map(|e| {
            let status = if e.up && e.carrier { "Connected" } else if e.up { "Up (no cable)" } else { "Down" };
            let name = e.name.clone();
            row![
                column![
                    label(e.name.clone()),
                    dim(format!("{status}{}", e.ip.as_deref().map(|ip| format!(" · {ip}")).unwrap_or_default())),
                ]
                .spacing(2)
                .width(Length::Fill),
                toggler(e.up).on_toggle(move |v| Message::EthernetToggled(name.clone(), v)),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(12)
            .into()
        }).collect()
    };

    let eth_card = container(
        column(
            std::iter::once(title("Ethernet"))
                .chain(std::iter::once(
                    dim("Toggling opens a terminal asking for your sudo password.".to_string())
                ))
                .chain(eth_rows)
                .collect::<Vec<_>>()
        )
        .spacing(12)
        .padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    // ── Bluetooth ────────────────────────────────────────────────────────
    let bt_scan_label: &'static str = if scanning { "Scanning…" } else { "Scan" };
    let bt_scan_btn = button(text(bt_scan_label).size(12).style(move |_| iced::widget::text::Style { color: Some(colors.text) }))
        .style(btn_style)
        .on_press_maybe(if scanning || !bluetooth_adapter.powered { None } else { Some(Message::BluetoothScanStart) })
        .padding([6, 14]);

    let mut bt_rows: Vec<Element<Message>> = Vec::new();
    if !bluetooth_adapter.present {
        bt_rows.push(dim("No Bluetooth adapter found.".to_string()));
    } else if bluetooth_adapter.powered {
        bt_rows.push(
            row![
                label("Discoverable".to_string()), iced::widget::horizontal_space(),
                toggler(bluetooth_adapter.discoverable).on_toggle(Message::BluetoothDiscoverableToggled),
            ]
            .align_y(iced::Alignment::Center).spacing(12).into()
        );
        bt_rows.push(
            row![
                label("Pairable".to_string()), iced::widget::horizontal_space(),
                toggler(bluetooth_adapter.pairable).on_toggle(Message::BluetoothPairableToggled),
            ]
            .align_y(iced::Alignment::Center).spacing(12).into()
        );

        if bluetooth_devices.is_empty() {
            bt_rows.push(dim("No devices found. Try Scan.".to_string()));
        } else {
            for d in bluetooth_devices {
                let status = if d.connected { "Connected" } else if d.paired { "Paired" } else { "Not paired" };
                let mac = d.mac.clone();

                let mut actions: Vec<Element<Message>> = Vec::new();
                if d.paired {
                    if d.connected {
                        actions.push(action_btn("Disconnect", Message::BluetoothDisconnect(mac.clone()), ActionKind::Plain));
                    } else {
                        actions.push(action_btn("Connect", Message::BluetoothConnect(mac.clone()), ActionKind::Accent));
                    }
                    actions.push(action_btn("Remove", Message::BluetoothRemove(mac.clone()), ActionKind::Danger));
                } else {
                    actions.push(action_btn("Pair", Message::BluetoothPair(mac.clone()), ActionKind::Accent));
                }

                bt_rows.push(
                    row![
                        column![
                            label(d.name.clone()),
                            dim(format!("{status} · {}", d.mac)),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        row(actions).spacing(8),
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10)
                    .into()
                );
            }
        }
    }

    let bt_card = container(
        column(
            std::iter::once(
                row![
                    title("Bluetooth"),
                    iced::widget::horizontal_space(),
                    bt_scan_btn,
                    toggler(bluetooth_adapter.powered).on_toggle(Message::BluetoothPowerToggled),
                ]
                .align_y(iced::Alignment::Center)
                .spacing(12)
                .into()
            )
            .chain(bt_rows)
            .collect::<Vec<_>>()
        )
        .spacing(12)
        .padding(20),
    )
    .style(card_style)
    .width(Length::Fill);

    scrollable(
        column![wifi_card, eth_card, bt_card]
            .spacing(16)
            .padding(20)
    )
    .into()
}
