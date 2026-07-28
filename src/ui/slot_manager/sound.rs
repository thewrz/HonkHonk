use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::state::SoundEntry;
use crate::ui::theme::{self, Hh, Theme};

use super::{tone_circle, tone_for};

pub(super) fn bound_tile<'a>(
    idx: u8,
    sound: &'a SoundEntry,
    trigger: Option<&'a str>,
    selected: bool,
    t: Theme,
) -> Element<'a, Message> {
    let tone = tone_for(sound);
    let bg = tone.tile_tint(t.is_dark());
    let border = if selected {
        iced::Border {
            color: t.ink(),
            width: 2.5,
            radius: 18.0.into(),
        }
    } else {
        iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: 18.0.into(),
        }
    };
    button(
        column![
            text(format!("#{:02}", idx + 1))
                .size(theme::font::LABEL)
                .color(t.ink_faint()),
            tone_circle(tone, 40.0, t),
            text(sound.name.clone())
                .size(theme::font::LABEL)
                .color(t.ink()),
            text(trigger.unwrap_or("no hotkey"))
                .size(theme::font::LABEL)
                .color(t.ink_faint()),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
        .padding(theme::space::SM),
    )
    .on_press(Message::SelectSlot(idx))
    .width(Length::Fill)
    .height(theme::component::SLOT_CARD_H)
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(bg)),
        text_color: t.ink(),
        border,
        ..Default::default()
    })
    .into()
}

pub(super) fn sound_header<'a>(sound: &'a SoundEntry, t: Theme) -> Element<'a, Message> {
    let tone = tone_for(sound);
    let circle = tone_circle(tone, 56.0, t);
    let info = column![
        text(sound.name.clone())
            .size(theme::font::BODY)
            .color(t.ink()),
        text(format!(
            "{} · {}",
            sound.category,
            crate::ui::fmt_duration(sound.duration_ms)
        ))
        .size(theme::font::LABEL)
        .color(t.ink_dim()),
    ]
    .spacing(2);
    row![circle, info]
        .spacing(theme::space::MD)
        .align_y(iced::Alignment::Center)
        .into()
}

pub(super) fn sidebar_bound_hotkey<'a>(trigger: Option<&'a str>, t: Theme) -> Element<'a, Message> {
    container(
        text(trigger.unwrap_or("—"))
            .size(theme::font::BODY)
            .color(t.ink()),
    )
    .padding([theme::space::SM, theme::space::MD])
    .width(Length::Fill)
    .style(move |_t| container::Style {
        border: iced::Border {
            color: t.accent(),
            width: 1.5,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}

pub(super) fn sidebar_bound_portal<'a>(t: Theme) -> Element<'a, Message> {
    let dot = container(Space::new())
        .width(theme::space::SM)
        .height(theme::space::SM)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(t.good())),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    container(
        row![
            dot,
            text("Registered via xdg-desktop-portal")
                .size(theme::font::LABEL)
                .color(t.ink_dim())
        ]
        .spacing(theme::space::SM)
        .align_y(iced::Alignment::Center),
    )
    .padding([theme::space::SM, theme::space::MD])
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.bg())),
        border: theme::tile_border(t.hairline(), 1.0),
        ..Default::default()
    })
    .into()
}

#[allow(
    clippy::too_many_lines,
    reason = "bound-slot sidebar keeps hotkey, portal status, and unbind controls in fixed order"
)]
pub(super) fn sidebar_bound<'a>(
    idx: u8,
    sound: &'a SoundEntry,
    trigger: Option<&'a str>,
    configure_available: bool,
    t: Theme,
) -> Element<'a, Message> {
    let slot_label = text(format!("SLOT #{:02}", idx + 1))
        .size(theme::font::LABEL)
        .color(t.ink_dim());
    let hk_display = sidebar_bound_hotkey(trigger, t);
    let portal = sidebar_bound_portal(t);
    let configure_row: Element<'_, Message> = if configure_available {
        button(
            text("Configure Shortcuts")
                .size(theme::font::LABEL)
                .color(t.ink()),
        )
        .on_press(Message::OpenShortcutConfig)
        .width(Length::Fill)
        .style(move |_t, _s| button::Style {
            background: Some(theme::bg_color(t.panel())),
            text_color: t.ink(),
            border: theme::tile_border(t.hairline(), 1.0),
            ..Default::default()
        })
        .into()
    } else {
        text("Assign keys in your desktop's shortcut settings")
            .size(theme::font::LABEL)
            .color(t.ink_faint())
            .into()
    };
    let unbind = button(
        text("Unbind")
            .size(theme::font::LABEL)
            .color(iced::Color::from_rgb(0.86, 0.15, 0.15)),
    )
    .on_press(Message::ClearSlot(idx))
    .width(Length::Fill)
    .style(move |_t, _s| button::Style {
        background: None,
        text_color: iced::Color::from_rgb(0.86, 0.15, 0.15),
        border: iced::Border {
            color: iced::Color::from_rgba(0.86, 0.15, 0.15, 0.4),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });
    column![
        slot_label,
        sound_header(sound, t),
        text("GLOBAL HOTKEY")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        hk_display,
        configure_row,
        text("PORTAL STATUS")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        portal,
        unbind,
    ]
    .spacing(theme::space::MD)
    .into()
}
