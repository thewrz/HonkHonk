mod audio;
mod common;
mod controls;
mod hotkeys;
mod library;
mod other;
mod scroll;

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, text},
};

use crate::app::{HonkHonk, Message, SettingsMessage, SettingsSection};
use crate::ui::theme::{self, Hh, Theme};
use crate::ui::{search_bar, settings::common::section_layout};

pub use controls::{get_setting_value, setting_message};
#[cfg(test)]
pub(crate) use scroll::highlighted_row_id;
pub(crate) use scroll::{content_scroll_id, locate_setting_row};

const SETTINGS_SECTIONS: &[SettingsSection] = &[
    SettingsSection::Audio,
    SettingsSection::Library,
    SettingsSection::Hotkeys,
    SettingsSection::Appearance,
    SettingsSection::About,
];

/// Top-level settings view — full window swap.
pub fn view_settings(state: &HonkHonk, t: Theme) -> Element<'_, Message> {
    let header = settings_header(t);
    let sidebar = settings_sidebar(state, t);
    let content = settings_content(state, t);
    let body = row![sidebar, content].height(Length::Fill);
    column![header, body]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_header(t: Theme) -> Element<'static, Message> {
    let back_btn = button(
        row![
            text("←").size(theme::font::BODY).color(t.ink()),
            text("Back to sounds")
                .size(theme::font::BODY)
                .color(t.ink()),
        ]
        .spacing(theme::space::SM)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ShowMain)
    .padding([8.0, 14.0])
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(t.panel())),
        border: theme::tile_border(t.hairline2(), 1.0),
        ..Default::default()
    });

    let title = row![
        text("Settings")
            .size(theme::font::TITLE)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .color(t.ink()),
        text("· ruffle feathers")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
    ]
    .spacing(theme::space::MD)
    .align_y(Alignment::Center);

    container(
        row![back_btn, title]
            .spacing(theme::space::LG)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([theme::space::MD, theme::space::XL])
    .style(move |_t| container::Style {
        border: iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: iced::border::Radius::default(),
        },
        ..Default::default()
    })
    .into()
}

fn settings_sidebar(state: &HonkHonk, t: Theme) -> Element<'_, Message> {
    let active = state.settings_ui.section();
    let nav = sidebar_categories(state)
        .iter()
        .copied()
        .fold(column![].spacing(theme::space::XS), |column, section| {
            column.push(sidebar_button(section, active, t))
        });

    let search = search_bar::view_settings_search_bar(state.settings_ui.query(), t, |query| {
        SettingsMessage::SearchChanged(query).into()
    });

    container(
        column![search, nav]
            .spacing(theme::space::MD)
            .width(Length::Fixed(220.0)),
    )
    .height(Length::Fill)
    .padding(theme::space::MD)
    .style(move |_t| container::Style {
        background: Some(theme::bg_color(t.panel())),
        border: iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: iced::border::Radius::default(),
        },
        ..Default::default()
    })
    .into()
}

fn sidebar_categories(state: &HonkHonk) -> &[SettingsSection] {
    if state.settings_ui.is_searching() {
        state.settings_ui.matching_categories()
    } else {
        SETTINGS_SECTIONS
    }
}

fn sidebar_button(
    section: SettingsSection,
    active: SettingsSection,
    t: Theme,
) -> Element<'static, Message> {
    let is_active = active == section;
    button(
        text(section.label())
            .size(theme::font::BODY)
            .color(if is_active { t.bg() } else { t.ink() })
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }),
    )
    .on_press(SettingsMessage::ShowSection(section).into())
    .width(Length::Fill)
    .padding([theme::space::SM, theme::space::MD])
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(if is_active {
            t.ink()
        } else {
            iced::Color::TRANSPARENT
        })),
        border: theme::tile_border(iced::Color::TRANSPARENT, 0.0),
        ..Default::default()
    })
    .into()
}

fn settings_content<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let body: Element<'_, Message> = if state.settings_ui.is_searching() {
        search_results(state, t)
    } else {
        match state.settings_ui.section() {
            SettingsSection::Audio => audio::view_audio_section(state, t),
            SettingsSection::Library => library::view_library_section(state, t),
            SettingsSection::Hotkeys => hotkeys::view_hotkeys_section(state, t),
            SettingsSection::Appearance => other::view_appearance_section(state, t),
            SettingsSection::About => other::view_about_section(t),
        }
    };

    scrollable(
        container(body)
            .width(Length::Fill)
            .padding([theme::space::XL, theme::space::XXL]),
    )
    .id(content_scroll_id())
    .on_scroll(|viewport| SettingsMessage::Scrolled(viewport.absolute_offset()).into())
    .height(Length::Fill)
    .into()
}

fn search_results<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let category = state.settings_ui.section();
    let rows = state.settings_ui.matching_settings();

    if rows.is_empty() {
        let message = if state.settings_ui.matching_categories().is_empty() {
            "No settings match your search."
        } else {
            "Choose a matching category from the sidebar."
        };
        return text(message)
            .size(theme::font::BODY)
            .color(t.ink_dim())
            .into();
    }

    let body = rows
        .iter()
        .copied()
        .fold(column![].spacing(theme::space::XS), |column, setting| {
            column.push(controls::render_setting_row(setting, state, t, true))
        });

    section_layout(
        category.label(),
        "Settings matching your search.",
        body.into(),
        t,
    )
}
