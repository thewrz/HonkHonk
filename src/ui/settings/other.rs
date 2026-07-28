use iced::{
    Element,
    widget::{column, container, text},
};

use super::common::section_layout;
use super::controls::render_setting_row;
use crate::app::{HonkHonk, Message};
use crate::settings::{SETTINGS_REGISTRY, SettingCategory};
use crate::ui::theme::{self, Hh, Theme};

const LICENSE: &str = env!("CARGO_PKG_LICENSE");

pub(super) fn view_appearance_section<'a>(state: &'a HonkHonk, t: Theme) -> Element<'a, Message> {
    let registry_rows = SETTINGS_REGISTRY
        .iter()
        .filter(|setting| setting.category == SettingCategory::Appearance)
        .fold(column![].spacing(0.0), |column, setting| {
            column.push(render_setting_row(setting, state, t, false))
        });

    section_layout(
        "Appearance",
        "How honky should HonkHonk look today?",
        registry_rows.into(),
        t,
    )
}

pub(super) fn view_about_section(t: Theme) -> Element<'static, Message> {
    section_layout(
        "About",
        "The bird is the word.",
        column![about_logo(t), license_badge(t), credits(t)]
            .spacing(theme::space::XL)
            .into(),
        t,
    )
}

fn about_logo(t: Theme) -> Element<'static, Message> {
    column![
        text("HonkHonk")
            .size(theme::font::HERO)
            .color(t.ink())
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                style: iced::font::Style::Italic,
                ..Default::default()
            }),
        text(format!("v{} · Iced 0.14", env!("CARGO_PKG_VERSION")))
            .size(theme::font::BODY)
            .color(t.ink_dim()),
        text("A Wayland-native soundboard for Linux. Built with Rust, Iced, and PipeWire.")
            .size(theme::font::LABEL)
            .color(t.ink_faint()),
    ]
    .spacing(theme::space::XS)
    .into()
}

fn license_badge(t: Theme) -> Element<'static, Message> {
    container(
        text(LICENSE)
            .size(theme::font::LABEL)
            .color(t.ink())
            .font(iced::Font {
                family: iced::font::Family::Monospace,
                ..Default::default()
            }),
    )
    .padding([4.0, 10.0])
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

fn credits(t: Theme) -> Element<'static, Message> {
    column![
        text("Credits")
            .size(theme::font::BODY)
            .color(t.ink())
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }),
        text("Iced — iced-rs")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        text("Symphonia — pdeljanov")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        text("ashpd — bilelmoussaoui")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        text("pipewire-rs — PipeWire project")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
        text("ksni — iovxw")
            .size(theme::font::LABEL)
            .color(t.ink_dim()),
    ]
    .spacing(theme::space::XS)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_license_matches_cargo_manifest() {
        assert_eq!(LICENSE, "MIT");
        assert_eq!(LICENSE, env!("CARGO_PKG_LICENSE"));
    }
}
