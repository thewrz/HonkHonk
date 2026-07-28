use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::theme::{self, Hh, Theme};

pub(super) fn empty_tile<'a>(idx: u8, selected: bool, t: Theme) -> Element<'a, Message> {
    let border = if selected {
        iced::Border {
            color: t.ink(),
            width: 2.5,
            radius: 18.0.into(),
        }
    } else {
        iced::Border {
            color: t.hairline2(),
            width: 2.0,
            radius: 18.0.into(),
        }
    };
    button(
        column![
            text(format!("#{:02}", idx + 1))
                .size(theme::font::LABEL)
                .color(t.ink_faint()),
            text("+").size(theme::font::TITLE).color(t.ink_faint()),
            text("EMPTY").size(theme::font::LABEL).color(t.ink_faint()),
        ]
        .spacing(6)
        .align_x(iced::Alignment::Center)
        .padding(theme::space::SM),
    )
    .on_press(Message::SelectSlot(idx))
    .width(Length::Fill)
    .height(theme::component::SLOT_CARD_H)
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(t.panel())),
        text_color: t.ink_faint(),
        border,
        ..Default::default()
    })
    .into()
}

pub(super) fn sidebar_empty<'a>(idx: u8, t: Theme) -> Element<'a, Message> {
    let slot_label = text(format!("SLOT #{:02}", idx + 1))
        .size(theme::font::LABEL)
        .color(t.ink_dim());
    let placeholder = container(
        column![
            text("🪿").size(32),
            text("Slot is empty").size(theme::font::BODY).color(t.ink()),
            text("Assign via right-click on any sound tile")
                .size(theme::font::LABEL)
                .color(t.ink_dim()),
        ]
        .spacing(theme::space::SM)
        .align_x(iced::Alignment::Center)
        .padding(theme::space::LG),
    )
    .width(Length::Fill)
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.bg())),
        border: iced::Border {
            color: t.hairline2(),
            width: 2.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    });
    column![slot_label, placeholder]
        .spacing(theme::space::MD)
        .into()
}
