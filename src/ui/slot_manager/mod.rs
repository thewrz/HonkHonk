use iced::widget::{Column, Row, Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

mod empty;
mod sound;

use crate::app::Message;
use crate::state::{SlotMap, SoundEntry};
use crate::ui::theme::{self, Hh, Theme, Tone};

/// Bundles the shared slot-manager view state to stay under clippy's
/// `too-many-arguments` threshold (5).
#[derive(Clone, Copy)]
pub struct SlotManagerCtx<'a> {
    pub slots: &'a SlotMap,
    pub slot_triggers: &'a [Option<String>; 20],
    pub sounds: &'a [SoundEntry],
    pub selected_slot: Option<u8>,
    /// Whether portal v2 `configure_shortcuts()` is available on this DE/backend.
    pub configure_available: bool,
}

pub(super) fn tone_for(sound: &SoundEntry) -> Tone {
    let idx = sound
        .id
        .get(..8)
        .and_then(|s| u64::from_str_radix(s, 16).ok())
        .unwrap_or(0) as usize;
    Tone::from_index(idx)
}

pub fn view_slot_manager<'a>(ctx: SlotManagerCtx<'a>, t: Theme) -> Element<'a, Message> {
    let bound_count = (0u8..20).filter(|&i| ctx.slots.get(i).is_some()).count();
    let header = slot_header(bound_count, t);
    let divider = container(Space::new())
        .width(1)
        .height(Length::Fill)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(t.hairline())),
            ..Default::default()
        });
    let grid = slot_grid(
        ctx.slots,
        ctx.slot_triggers,
        ctx.sounds,
        ctx.selected_slot,
        t,
    );
    let side = sidebar(ctx, t);
    let body = row![grid, divider, side].height(Length::Fill);
    container(column![header, body].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(t.bg())),
            ..Default::default()
        })
        .into()
}

fn slot_header<'a>(bound_count: usize, t: Theme) -> Element<'a, Message> {
    let back_btn = button(
        row![
            text("←").size(theme::font::BODY).color(t.ink()),
            text("Back to sounds")
                .size(theme::font::BODY)
                .color(t.ink()),
        ]
        .spacing(theme::space::XS)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::ShowMain)
    .style(move |_t, _s| button::Style {
        background: Some(theme::bg_color(t.panel())),
        text_color: t.ink(),
        border: theme::tile_border(t.hairline(), 1.0),
        ..Default::default()
    });

    let title = text("Slots").size(theme::font::TITLE).color(t.ink());
    let sep = text("·").size(theme::font::BODY).color(t.ink_dim());
    let stats = text(format!("{bound_count} bound"))
        .size(theme::font::LABEL)
        .color(t.ink_dim());

    container(
        row![back_btn, title, sep, stats]
            .spacing(theme::space::MD)
            .align_y(iced::Alignment::Center),
    )
    .padding([theme::space::MD, theme::space::LG])
    .style(move |_t| container::Style {
        border: iced::Border {
            color: t.hairline(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn slot_grid<'a>(
    slots: &'a SlotMap,
    slot_triggers: &'a [Option<String>; 20],
    sounds: &'a [SoundEntry],
    selected_slot: Option<u8>,
    t: Theme,
) -> Element<'a, Message> {
    let rows: Vec<Element<'_, Message>> = (0u8..4)
        .map(|row_idx| {
            let tiles: Vec<Element<'_, Message>> = (0u8..5)
                .map(|col_idx| {
                    let idx = row_idx * 5 + col_idx;
                    let sound = slots
                        .get(idx)
                        .and_then(|p| sounds.iter().find(|s| &s.path == p));
                    slot_tile(idx, sound, slot_triggers, selected_slot == Some(idx), t)
                })
                .collect();
            Row::with_children(tiles).spacing(theme::space::MD).into()
        })
        .collect();

    scrollable(
        container(Column::with_children(rows).spacing(theme::space::MD))
            .padding(theme::space::LG)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn slot_tile<'a>(
    idx: u8,
    sound: Option<&'a SoundEntry>,
    slot_triggers: &'a [Option<String>; 20],
    selected: bool,
    t: Theme,
) -> Element<'a, Message> {
    match sound {
        Some(s) => sound::bound_tile(idx, s, slot_triggers[idx as usize].as_deref(), selected, t),
        None => empty::empty_tile(idx, selected, t),
    }
}

pub(super) fn tone_circle<'a>(tone: Tone, size: f32, t: Theme) -> Element<'a, Message> {
    let r = size / 2.0;
    container(Space::new())
        .width(size)
        .height(size)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(tone.highlight(t.is_dark()))),
            border: iced::Border {
                radius: r.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn sidebar<'a>(ctx: SlotManagerCtx<'a>, t: Theme) -> Element<'a, Message> {
    let inner: Element<'_, Message> = match ctx.selected_slot {
        None => text("Select a slot to inspect it")
            .size(theme::font::BODY)
            .color(t.ink_faint())
            .into(),
        Some(idx) => {
            let sound = ctx
                .slots
                .get(idx)
                .and_then(|p| ctx.sounds.iter().find(|s| &s.path == p));
            match sound {
                Some(s) => {
                    let trigger = ctx
                        .slot_triggers
                        .get(idx as usize)
                        .and_then(|t| t.as_deref());
                    sound::sidebar_bound(idx, s, trigger, ctx.configure_available, t)
                }
                None => empty::sidebar_empty(idx, t),
            }
        }
    };
    container(inner)
        .width(320)
        .height(Length::Fill)
        .padding(theme::space::LG)
        .style(move |_t| container::Style {
            background: Some(theme::bg_color(t.panel())),
            ..Default::default()
        })
        .into()
}
