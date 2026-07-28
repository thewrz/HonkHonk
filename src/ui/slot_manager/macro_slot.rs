//! Macro-slot tile and sidebar rendering (#169). Mirrors `sound.rs`'s
//! bound-slot layout, swapping the tone circle for an accent-colored "MACRO"
//! badge and the sound header for a name + step-count summary.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::state::Macro;
use crate::ui::theme::{self, Hh, Theme};

use super::{controls, display_name};

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
            text(display_name(macro_def))
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
            text(display_name(macro_def))
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

/// Mirrors `sound::sidebar_bound`'s control order exactly — a macro slot's
/// hotkey is configured the same app-global way a sound slot's is, so it
/// gets the same hotkey readout, configure affordance and portal status
/// (#169 review).
pub(super) fn sidebar_bound<'a>(
    idx: u8,
    macro_def: &'a Macro,
    trigger: Option<&'a str>,
    configure_available: bool,
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
