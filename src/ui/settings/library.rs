use iced::{
    Alignment, Element, Length,
    widget::{Column, Row, button, column, container, row, text},
};

use super::common::section_layout;
use super::controls::render_setting_row;
use crate::app::{HonkHonk, Message};
use crate::settings::{SETTINGS_REGISTRY, SettingCategory};
use crate::ui::theme::{self, Hh, Theme};

pub(super) fn view_library_section<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let registry_rows = SETTINGS_REGISTRY
        .iter()
        .filter(|setting| setting.category == SettingCategory::Library)
        .fold(column![].spacing(0.0), |column, setting| {
            column.push(render_setting_row(setting, state, t, false))
        });

    section_layout(
        "Library",
        "Where HonkHonk looks for your sounds.",
        column![
            folders_row(state, t),
            registry_rows,
            supported_formats_row(t)
        ]
        .spacing(theme::space::LG)
        .into(),
        t,
    )
}

fn folders_row<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let folder_rows: Vec<Element<'_, Message>> = state
        .config
        .sound_directories
        .iter()
        .map(|path| folder_row(path, t))
        .collect();
    let add_button = button(
        text("+ Add a folder")
            .size(theme::font::BODY)
            .color(t.ink_dim()),
    )
    .on_press(Message::AddSoundDirectory)
    .width(Length::Fill)
    .padding([9.0, 12.0])
    .style(move |_t, _s| button::Style {
        background: None,
        border: iced::Border {
            color: t.hairline2(),
            width: 1.5,
            radius: theme::radius::MD,
        },
        ..Default::default()
    });
    let folders = column![
        Column::with_children(folder_rows).spacing(theme::space::XS),
        add_button,
    ]
    .spacing(theme::space::XS)
    .width(Length::Fixed(540.0));

    row![
        row_label(
            "Sound folders",
            "HonkHonk watches these folders. Drop in MP3 / WAV / OGG / FLAC.",
            t
        ),
        folders,
    ]
    .spacing(theme::space::XL)
    .align_y(Alignment::Start)
    .width(Length::Fill)
    .into()
}

fn folder_row(path: &std::path::Path, t: Theme) -> Element<'static, Message> {
    let remove_button = button(text("×").size(theme::font::BODY).color(t.ink_faint()))
        .on_press(Message::RemoveSoundDirectory(path.to_path_buf()))
        .padding(4.0)
        .style(move |_t, _s| button::Style {
            background: None,
            border: iced::Border::default(),
            ..Default::default()
        });

    container(
        row![
            text(path.display().to_string())
                .size(theme::font::LABEL)
                .color(t.ink())
                .font(iced::Font {
                    family: iced::font::Family::Monospace,
                    ..Default::default()
                })
                .width(Length::Fill),
            remove_button,
        ]
        .spacing(theme::space::SM)
        .align_y(Alignment::Center),
    )
    .padding([10.0, 12.0])
    .width(Length::Fill)
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

fn supported_formats_row(t: Theme) -> Element<'static, Message> {
    const FORMATS: &[&str] = &["MP3", "WAV", "OGG Vorbis", "FLAC", "AAC", "Opus"];
    let format_pills: Vec<Element<'_, Message>> = FORMATS
        .iter()
        .map(|format| {
            container(
                text(*format)
                    .size(theme::font::LABEL)
                    .color(t.ink_dim())
                    .font(iced::Font {
                        family: iced::font::Family::Monospace,
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }),
            )
            .padding([5.0, 11.0])
            .style(move |_t| container::Style {
                background: Some(theme::bg_color(t.panel())),
                border: iced::Border {
                    color: t.hairline2(),
                    width: 1.0,
                    radius: theme::radius::PILL,
                },
                ..Default::default()
            })
            .into()
        })
        .collect();

    row![
        row_label("Supported formats", "Decoded via Symphonia — pure Rust.", t),
        Row::with_children(format_pills).spacing(theme::space::XS),
    ]
    .spacing(theme::space::XL)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn row_label(label: &'static str, hint: &'static str, t: Theme) -> Element<'static, Message> {
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
