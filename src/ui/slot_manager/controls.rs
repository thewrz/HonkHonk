//! Sidebar controls shared by every bound-slot renderer, regardless of the
//! slot's content kind (#169 review). `sound.rs` and `macro_slot.rs` render
//! the same hotkey readout, shortcut-configuration affordance, portal status
//! and unbind control; keeping one copy here is what stops the two sidebars
//! from drifting apart the way the configure control already had.

use iced::widget::{Space, button, container, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::theme::{self, Hh, Theme};

/// Read-only display of the slot's currently bound global hotkey, or an
/// em dash when the portal has not reported one.
pub(super) fn hotkey_display<'a>(trigger: Option<&'a str>, t: Theme) -> Element<'a, Message> {
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

/// Opens the desktop's shortcut editor when portal v2 exposes
/// `configure_shortcuts()`, otherwise points the user at their DE's own
/// settings. Shortcut configuration is app-global rather than per-slot, so
/// every bound slot offers it: rendering it only for sound slots left a user
/// whose slots were all macros with no route to it at all (#169 review).
pub(super) fn configure_row<'a>(configure_available: bool, t: Theme) -> Element<'a, Message> {
    if !configure_available {
        return text("Assign keys in your desktop's shortcut settings")
            .size(theme::font::LABEL)
            .color(t.ink_faint())
            .into();
    }
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
}

/// Confirms the slot's shortcut is registered through xdg-desktop-portal.
pub(super) fn portal_status<'a>(t: Theme) -> Element<'a, Message> {
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

/// Releases the slot. [`Message::ClearSlot`] clears either content kind, so
/// one control serves sound and macro slots alike.
pub(super) fn unbind_button<'a>(idx: u8) -> Element<'a, Message> {
    let danger = iced::Color::from_rgb(0.86, 0.15, 0.15);
    button(text("Unbind").size(theme::font::LABEL).color(danger))
        .on_press(Message::ClearSlot(idx))
        .width(Length::Fill)
        .style(move |_t, _s| button::Style {
            background: None,
            text_color: danger,
            border: iced::Border {
                color: iced::Color::from_rgba(0.86, 0.15, 0.15, 0.4),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}
