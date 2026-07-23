use iced::{
    Element, Length,
    widget::{Space, column, container, text},
};

use crate::app::Message;
use crate::ui::theme::{self, Hh, Theme};

pub(super) fn label_hint_column(
    label: &'static str,
    hint: &'static str,
    t: Theme,
) -> Element<'static, Message> {
    column![
        text(label)
            .size(theme::font::BODY)
            .color(t.ink())
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }),
        text(hint).size(theme::font::LABEL).color(t.ink_dim()),
    ]
    .spacing(theme::space::XS)
    .width(Length::Fixed(260.0))
    .into()
}

/// Shared section chrome: bold italic title + subtitle + 2px ink underline + body.
pub(super) fn section_layout<'a>(
    title: &'static str,
    subtitle: &'static str,
    body: Element<'a, Message>,
    t: Theme,
) -> Element<'a, Message> {
    column![
        column![
            text(title)
                .size(theme::font::TITLE)
                .color(t.ink())
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    style: iced::font::Style::Italic,
                    ..Default::default()
                }),
            text(subtitle).size(theme::font::BODY).color(t.ink_dim()),
        ]
        .spacing(theme::space::XS)
        .width(Length::Fill),
        container(Space::new())
            .width(Length::Fill)
            .height(2)
            .style(move |_t| container::Style {
                background: Some(theme::bg_color(t.ink())),
                ..Default::default()
            }),
        body,
    ]
    .spacing(theme::space::LG)
    .width(Length::Fill)
    .into()
}
