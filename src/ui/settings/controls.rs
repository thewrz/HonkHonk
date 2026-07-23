use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text},
};

use crate::app::{HonkHonk, Message};
use crate::settings::{ControlType, SettingDef, SettingId, SettingValue};
use crate::ui::theme::{self, Hh, Theme};

/// Generic registry row renderer.
pub(super) fn render_setting_row<'a>(
    def: &'a SettingDef,
    state: &'a HonkHonk,
    t: Theme,
    highlighted: bool,
) -> Element<'a, Message> {
    let value = get_setting_value(def.id, state);
    let label_col = setting_label(def, t);
    let control = setting_control(def, value, t);

    container(
        row![label_col, control]
            .spacing(theme::space::XL)
            .align_y(Alignment::Start)
            .width(Length::Fill),
    )
    .id(super::scroll::row_id(def.id))
    .width(Length::Fill)
    .padding([18.0, 10.0])
    .style(move |_t| container::Style {
        background: highlighted.then(|| {
            theme::bg_color(iced::Color {
                a: 0.12,
                ..t.accent()
            })
        }),
        border: theme::tile_border(
            if highlighted {
                t.accent()
            } else {
                iced::Color::TRANSPARENT
            },
            if highlighted { 1.0 } else { 0.0 },
        ),
        ..Default::default()
    })
    .into()
}

fn setting_label(def: &SettingDef, t: Theme) -> Element<'_, Message> {
    column![
        text(def.label)
            .size(theme::font::BODY)
            .color(t.ink())
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            }),
        text(def.hint).size(theme::font::LABEL).color(t.ink_dim()),
    ]
    .spacing(theme::space::XS)
    .width(Length::Fixed(260.0))
    .into()
}

fn setting_control(def: &SettingDef, value: SettingValue, t: Theme) -> Element<'_, Message> {
    match (&def.control, value) {
        (ControlType::Button, _) => render_button(def.id, def.label, t),
        (ControlType::Radio(options), SettingValue::Index(current)) => {
            render_radio(def.id, options, current, t)
        }
        (ControlType::Toggle, SettingValue::Bool(value)) => render_toggle(def.id, value, t),
        (ControlType::Slider { min, max, step }, SettingValue::F32(value)) => {
            render_slider(def.id, value, (*min, *max, *step), t)
        }
        _ => text("—")
            .size(theme::font::BODY)
            .color(t.ink_faint())
            .into(),
    }
}

fn render_button(id: SettingId, label: &'static str, t: Theme) -> Element<'static, Message> {
    button(text(label).size(theme::font::BODY).color(t.ink()))
        .on_press(tracked_setting_message(id, SettingValue::None))
        .padding([8.0, 18.0])
        .style(move |_t, _s| button::Style {
            background: Some(theme::bg_color(t.panel())),
            border: theme::tile_border(t.hairline2(), 1.0),
            ..Default::default()
        })
        .into()
}

fn render_radio(
    id: SettingId,
    options: &'static [&'static str],
    current: usize,
    t: Theme,
) -> Element<'static, Message> {
    options
        .iter()
        .enumerate()
        .fold(row![].spacing(theme::space::XS), |row, (index, label)| {
            let active = index == current;
            row.push(
                button(text(*label).size(theme::font::BODY).color(if active {
                    t.bg()
                } else {
                    t.ink()
                }))
                .on_press(tracked_setting_message(id, SettingValue::Index(index)))
                .padding([6.0, 14.0])
                .style(move |_t, _s| button::Style {
                    background: Some(theme::bg_color(if active { t.ink() } else { t.panel() })),
                    border: theme::tile_border(t.hairline2(), 1.0),
                    ..Default::default()
                }),
            )
        })
        .into()
}

pub fn get_setting_value(id: SettingId, state: &HonkHonk) -> SettingValue {
    match id {
        SettingId::RescanLibrary => SettingValue::None,
        SettingId::Theme => SettingValue::Index(state.config.theme.setting_index()),
        SettingId::Density => SettingValue::Index(state.config.density.setting_index()),
        SettingId::PanelAnimations => SettingValue::Bool(state.config.panel_animations),
        SettingId::MicPassthrough => SettingValue::Bool(state.config.mic_passthrough),
        SettingId::MicPassthroughLevel => SettingValue::F32(state.config.mic_passthrough_level),
        SettingId::OverlapMode => SettingValue::Index(state.config.overlap_mode.setting_index()),
        SettingId::Renderer => {
            SettingValue::Bool(state.config.renderer == crate::state::Renderer::Wgpu)
        }
        _ => SettingValue::None,
    }
}

pub fn setting_message(id: SettingId, value: SettingValue) -> Message {
    match (id, value) {
        (SettingId::RescanLibrary, _) => Message::RescanLibrary,
        (SettingId::Theme, SettingValue::Index(index)) => {
            Message::ThemeChanged(crate::ui::theme::Theme::from_setting_index(index))
        }
        (SettingId::Density, SettingValue::Index(index)) => {
            Message::DensityChanged(crate::state::config::Density::from_setting_index(index))
        }
        (SettingId::PanelAnimations, SettingValue::Bool(value)) => {
            Message::PanelAnimationsChanged(value)
        }
        (SettingId::MicPassthrough, SettingValue::Bool(value)) => {
            Message::MicPassthroughChanged(value)
        }
        (SettingId::MicPassthroughLevel, SettingValue::F32(value)) => {
            Message::MicPassthroughLevelChanged(value)
        }
        (SettingId::OverlapMode, SettingValue::Index(index)) => {
            Message::OverlapModeChanged(crate::state::OverlapMode::from_setting_index(index))
        }
        (SettingId::Renderer, SettingValue::Bool(value)) => Message::RendererChanged(if value {
            crate::state::Renderer::Wgpu
        } else {
            crate::state::Renderer::TinySkia
        }),
        other => {
            tracing::error!(?other, "setting_message: unhandled setting/value combo");
            Message::NoOp
        }
    }
}

fn tracked_setting_message(id: SettingId, value: SettingValue) -> Message {
    Message::SettingInteracted {
        id,
        action: Box::new(setting_message(id, value)),
    }
}

fn render_toggle(id: SettingId, value: bool, t: Theme) -> Element<'static, Message> {
    row![
        toggle_button(id, true, value, "On", t),
        toggle_button(id, false, !value, "Off", t),
    ]
    .spacing(theme::space::XS)
    .into()
}

fn toggle_button(
    id: SettingId,
    value: bool,
    active: bool,
    label: &'static str,
    t: Theme,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(theme::font::BODY)
            .color(if active { t.bg() } else { t.ink() }),
    )
    .on_press(tracked_setting_message(id, SettingValue::Bool(value)))
    .padding([6.0, 14.0])
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(if active { t.ink() } else { t.panel() })),
        border: theme::tile_border(t.hairline2(), 1.0),
        ..Default::default()
    })
    .into()
}

fn render_slider(
    id: SettingId,
    value: f32,
    range: (f32, f32, f32),
    t: Theme,
) -> Element<'static, Message> {
    let (min, max, step) = range;
    row![
        iced::widget::slider(min..=max, value, move |next| {
            tracked_setting_message(id, SettingValue::F32(next))
        })
        .step(step)
        .width(Length::Fixed(200.0)),
        text(format!("{:.0}%", value * 100.0))
            .size(theme::font::LABEL)
            .color(t.ink_dim())
            .font(iced::Font {
                family: iced::font::Family::Monospace,
                ..Default::default()
            }),
    ]
    .spacing(theme::space::SM)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SETTINGS_REGISTRY;

    #[test]
    fn invalid_setting_value_never_triggers_a_real_action() {
        let result = std::panic::catch_unwind(|| {
            setting_message(SettingId::Theme, SettingValue::Bool(true))
        });
        let message = result.expect("invalid setting values must degrade safely");

        assert_eq!(message, Message::NoOp);
    }

    #[test]
    fn every_registry_control_has_a_message_mapping() {
        for setting in SETTINGS_REGISTRY {
            let value = representative_value(setting.control);
            let message = setting_message(setting.id, value);

            assert!(
                !matches!(message, Message::NoOp),
                "{:?} has no setting-message mapping",
                setting.id
            );
        }
    }

    fn representative_value(control: ControlType) -> SettingValue {
        match control {
            ControlType::Toggle => SettingValue::Bool(true),
            ControlType::Radio(_) => SettingValue::Index(0),
            ControlType::Slider { min, .. } => SettingValue::F32(min),
            ControlType::Button => SettingValue::None,
            ControlType::Select => SettingValue::None,
        }
    }
}
