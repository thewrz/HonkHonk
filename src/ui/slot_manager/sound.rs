use iced::widget::{button, column, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::state::SoundEntry;
use crate::ui::theme::{self, Hh, Theme};

use super::{controls, tone_circle, tone_for};

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
    column![
        slot_label,
        sound_header(sound, t),
        text("GLOBAL HOTKEY")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        controls::hotkey_display(trigger, t),
        controls::configure_row(configure_available, t),
        text("PORTAL STATUS")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        controls::portal_status(t),
        controls::unbind_button(idx),
    ]
    .spacing(theme::space::MD)
    .into()
}
