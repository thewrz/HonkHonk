//! Macro-slot tile and sidebar rendering (#169). Mirrors `sound.rs`'s
//! bound-slot layout, swapping the tone circle for an accent-colored "MACRO"
//! badge and the sound header for a name + step-count summary.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::state::Macro;
use crate::ui::theme::{self, Hh, Theme};

fn macro_badge<'a>(t: Theme) -> Element<'a, Message> {
    container(
        text("MACRO")
            .size(theme::font::LABEL)
            .color(iced::Color::from_rgb(0.1, 0.07, 0.03)),
    )
    .padding([2.0, theme::space::SM])
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.accent())),
        border: iced::Border {
            radius: theme::radius::PILL,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

pub(super) fn tile<'a>(
    idx: u8,
    macro_def: &'a Macro,
    trigger: Option<&'a str>,
    selected: bool,
    t: Theme,
) -> Element<'a, Message> {
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
            macro_badge(t),
            text(macro_def.name.clone())
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
        background: Some(theme::bg_color(t.panel())),
        text_color: t.ink(),
        border,
        ..Default::default()
    })
    .into()
}

fn macro_summary<'a>(macro_def: &'a Macro, t: Theme) -> Element<'a, Message> {
    row![
        macro_badge(t),
        column![
            text(macro_def.name.clone())
                .size(theme::font::BODY)
                .color(t.ink()),
            text(format!("{} step(s)", macro_def.steps.len()))
                .size(theme::font::LABEL)
                .color(t.ink_dim()),
        ]
        .spacing(2),
    ]
    .spacing(theme::space::MD)
    .align_y(iced::Alignment::Center)
    .into()
}

fn hotkey_display<'a>(trigger: Option<&'a str>, t: Theme) -> Element<'a, Message> {
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

fn unbind_button<'a>(idx: u8) -> Element<'a, Message> {
    button(
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
    })
    .into()
}

pub(super) fn sidebar_bound<'a>(
    idx: u8,
    macro_def: &'a Macro,
    trigger: Option<&'a str>,
    t: Theme,
) -> Element<'a, Message> {
    let slot_label = text(format!("SLOT #{:02}", idx + 1))
        .size(theme::font::LABEL)
        .color(t.ink_dim());
    column![
        slot_label,
        macro_summary(macro_def, t),
        text("GLOBAL HOTKEY")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        hotkey_display(trigger, t),
        unbind_button(idx),
    ]
    .spacing(theme::space::MD)
    .into()
}
