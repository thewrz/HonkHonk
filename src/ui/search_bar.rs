use iced::widget::{button, container, row, text, text_input};
use iced::{Alignment, Border, Element, Length, Padding};

use crate::ui::theme::{self, Hh, Theme};

const INPUT_ID: &str = "honkhonk-shared-filter";
const SETTINGS_INPUT_ID: &str = "honkhonk-settings-filter";

/// Returns the stable widget identifier used for programmatic filter focus.
pub fn input_id() -> iced::widget::Id {
    iced::widget::Id::new(INPUT_ID)
}

#[allow(
    clippy::too_many_lines,
    reason = "stable stack layout avoids Iced text-input focus reset across query states"
)]
/// Builds the shared search input using the caller's message mapper.
pub fn view_search_bar<'a, Message>(
    query: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    view_search_input(
        "Find a sound to honk\u{2026}",
        query,
        input_id(),
        Length::Fixed(300.0),
        on_input,
    )
}

/// Builds the click-only settings search using the same stable input stack.
pub fn view_settings_search_bar<'a, Message>(
    query: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    view_search_input(
        "Search settings\u{2026}",
        query,
        iced::widget::Id::new(SETTINGS_INPUT_ID),
        Length::Fill,
        on_input,
    )
}

fn view_search_input<'a, Message>(
    placeholder: &'a str,
    query: &'a str,
    id: iced::widget::Id,
    width: Length,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let clear_message = on_input(String::new());
    let input = search_input(placeholder, query, id, width, on_input);
    let overlay = clear_overlay(query, clear_message, width);
    iced::widget::stack![input, overlay].into()
}

fn search_input<'a, Message>(
    placeholder: &'a str,
    query: &'a str,
    id: iced::widget::Id,
    width: Length,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let t = Theme::Dark;
    // Reserve right space for the clear button so typed text doesn't run under it.
    let padding = if query.is_empty() {
        Padding::from(5.0)
    } else {
        Padding {
            top: 5.0,
            right: 30.0,
            bottom: 5.0,
            left: 10.0,
        }
    };

    text_input(placeholder, query)
        .id(id)
        .on_input(on_input)
        .size(theme::font::BODY)
        .width(width)
        .padding(padding)
        .style(move |_theme, status| {
            let border_color = match status {
                text_input::Status::Focused { .. } => t.accent(),
                _ => t.hairline(),
            };
            text_input::Style {
                background: theme::bg_color(t.panel()),
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: theme::radius::PILL,
                },
                icon: t.ink_dim(),
                placeholder: t.ink_faint(),
                value: t.ink(),
                selection: t.accent(),
            }
        })
        .into()
}

fn clear_overlay<'a, Message>(
    query: &str,
    clear_message: Message,
    width: Length,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let t = Theme::Dark;
    // Always use stack so the widget tree shape is stable across all query states.
    // Changing from container → stack on first keystroke caused Iced to reset
    // text_input focus. An empty row as the second layer has no hit area or cost.
    if query.is_empty() {
        row![].into()
    } else {
        // Clear button — floats over the right edge of the input via stack.
        let clear_btn = button(text("\u{2715}").size(theme::font::BODY).color(t.ink_dim()))
            .on_press(clear_message)
            .padding(Padding {
                top: 4.0,
                right: 10.0,
                bottom: 4.0,
                left: 4.0,
            })
            .style(move |_t, status| button::Style {
                text_color: match status {
                    button::Status::Hovered | button::Status::Pressed => t.ink(),
                    _ => t.ink_dim(),
                },
                background: None,
                ..Default::default()
            });

        container(clear_btn)
            .width(width)
            .align_x(Alignment::End)
            .align_y(Alignment::Center)
            .into()
    }
}
