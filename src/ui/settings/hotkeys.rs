//! Settings → Shortcuts section view (#199): search bar + sort chip over the
//! pure `hotkey_rows()` query surface, plus the global-shortcuts portal
//! status badge. Extracted from `other.rs` once the row model, filter/sort
//! state, and cross-module-tree accessors existed to rewire it onto them.

use iced::widget::{Column, column, container, row, space, text};
use iced::{Alignment, Element, Length};

use super::common::section_layout;
use crate::app::{HonkHonk, HotkeyRow, Message};
use crate::shortcuts::ShortcutsStatus;
use crate::ui::list_controls::sort;
use crate::ui::search_bar;
use crate::ui::theme::{self, Hh, Theme};

pub(super) fn view_hotkeys_section<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let search = search_bar::view_hotkeys_search_bar(
        state.hotkey_filter_query(),
        t,
        Message::HotkeySearchChanged,
    );
    let sort = sort::view_sort_chip(
        state.hotkey_sort_state(),
        Message::ToggleHotkeySortMenu,
        Message::ToggleHotkeySortDirection,
        t,
    );
    let controls = row![search, sort]
        .spacing(theme::space::SM)
        .align_y(Alignment::Center);

    section_layout(
        "Hotkeys",
        "Global shortcuts that work even when HonkHonk isn't focused.",
        column![
            controls,
            portal_status_badge(state, t),
            hotkey_bindings(state, t)
        ]
        .spacing(theme::space::LG)
        .into(),
        t,
    )
}

/// The global-shortcuts portal connection indicator: a colored dot plus
/// status text. Extracted from `view_hotkeys_section` to keep it under the
/// `too_many_lines` clippy budget once the search bar and sort chip landed.
fn portal_status_badge(state: &HonkHonk, t: Theme) -> Element<'static, Message> {
    let (dot_color, status_text) = match &state.shortcuts_status {
        ShortcutsStatus::Active => (t.good(), "Global shortcuts active"),
        ShortcutsStatus::Initializing => (t.ink_dim(), "Connecting to portal…"),
        ShortcutsStatus::Unavailable(_) => (t.accent(), "Portal unavailable"),
    };
    let dot = container(iced::widget::Space::new())
        .width(theme::space::SM)
        .height(theme::space::SM)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(dot_color)),
            border: iced::Border {
                radius: iced::border::Radius::from(4.0),
                ..Default::default()
            },
            ..Default::default()
        });

    container(
        row![
            dot,
            text(status_text)
                .size(theme::font::LABEL)
                .color(t.ink())
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                }),
        ]
        .spacing(theme::space::SM)
        .align_y(Alignment::Center),
    )
    .padding(theme::space::MD)
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.panel())),
        border: iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: theme::radius::MD,
        },
        ..Default::default()
    })
    .into()
}

/// The bound-shortcut list, sourced from the pure, filtered, sorted
/// `hotkey_rows()` query surface rather than raw `slot_triggers`. Two
/// distinct empty-state messages: nothing bound at all versus a filter query
/// that matched nothing.
fn hotkey_bindings<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let rows = state.hotkey_rows();

    if rows.is_empty() {
        let message = if state.hotkey_filter_query().is_empty() {
            "No hotkeys assigned yet. Use the Slot Manager to bind sounds."
        } else {
            "No hotkeys match your search."
        };
        return text(message)
            .size(theme::font::LABEL)
            .color(t.ink_dim())
            .into();
    }

    let elements: Vec<Element<'static, Message>> =
        rows.iter().map(|row| hotkey_row(row, t)).collect();
    Column::with_children(elements)
        .spacing(theme::space::XS)
        .into()
}

/// One bound-shortcut row. Explicit `'static` output lifetime, not the
/// elided `&HotkeyRow`-borrowed lifetime: the body only clones owned
/// `String`s out of `row` into the widgets, so `'static` is sound, and it
/// keeps the `Vec<Element>` in `hotkey_bindings` from illegally borrowing the
/// function-local `Vec<HotkeyRow>` that `hotkey_rows()` returns.
fn hotkey_row(row: &HotkeyRow, t: Theme) -> Element<'static, Message> {
    let slot_label = text(format!("Slot {}", row.slot_index + 1))
        .size(theme::font::LABEL)
        .color(t.ink_dim())
        .width(Length::Fixed(60.0));

    let name = if row.tag.is_empty() {
        column![row_title(&row.display_name, t)]
    } else {
        column![row_title(&row.display_name, t), row_tag(&row.tag, t)]
    }
    .spacing(2.0);

    let trigger = text(row.trigger.clone())
        .size(theme::font::LABEL)
        .color(t.ink())
        .font(iced::Font {
            family: iced::font::Family::Monospace,
            weight: iced::font::Weight::Bold,
            ..Default::default()
        });

    container(
        row![slot_label, name, space::horizontal(), trigger]
            .spacing(theme::space::MD)
            .align_y(Alignment::Center),
    )
    .padding([6.0, 12.0])
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.panel())),
        border: iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: theme::radius::MD,
        },
        ..Default::default()
    })
    .into()
}

fn row_title(display_name: &str, t: Theme) -> Element<'static, Message> {
    text(display_name.to_owned())
        .size(theme::font::LABEL)
        .color(t.ink())
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        })
        .into()
}

fn row_tag(tag: &str, t: Theme) -> Element<'static, Message> {
    text(tag.to_owned())
        .size(theme::font::LABEL)
        .color(t.ink_faint())
        .into()
}

#[cfg(test)]
mod tests {
    use crate::app::HonkHonk;
    use crate::ui::theme::Theme;

    use super::view_hotkeys_section;

    /// Smoke coverage only (per `CLAUDE.md`, Iced view rendering itself is
    /// not tested here): the section must build without panicking for the
    /// unfiltered-empty state (no hotkeys assigned yet), matching the sibling
    /// `library`/`audio` section views' convention of leaving pixel-level
    /// assertions to the invariant tests at `hotkey_rows()`'s boundary.
    #[test]
    fn view_hotkeys_section_builds_for_the_default_state() {
        let state = HonkHonk::new_for_test();

        let _element = view_hotkeys_section(&state, Theme::Light);
    }
}
