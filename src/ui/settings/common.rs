use iced::{
    Element, Length,
    widget::{Space, column, container, text},
};

use crate::app::Message;
use crate::ui::theme::{self, Hh, Theme};

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
