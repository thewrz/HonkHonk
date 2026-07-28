//! Boundary tests for the render-time slot-view resolution (#169 Task 3),
//! split from `mod.rs` to keep it within the file-size budget. Pins:
//! `resolve_slot` is pure and never self-clears a dangling reference, and
//! `bound_count` counts a slot as bound for either `SlotContent` variant.

use super::*;
use crate::state::{AudioFormat, SlotMap};
use std::path::PathBuf;

fn sound(id: &str, path: &str) -> SoundEntry {
    SoundEntry {
        id: id.into(),
        name: id.to_uppercase(),
        path: PathBuf::from(path),
        format: AudioFormat::Wav,
        duration_ms: Some(100),
        modified_ms: None,
        category: "Test".into(),
    }
}

fn macro_def(id: &str, name: &str) -> Macro {
    Macro {
        id: id.into(),
        name: name.into(),
        steps: Vec::new(),
    }
}

fn empty_triggers() -> [Option<String>; 20] {
    std::array::from_fn(|_| None)
}

fn ctx<'a>(
    slots: &'a SlotMap,
    slot_triggers: &'a [Option<String>; 20],
    sounds: &'a [SoundEntry],
    macros: &'a MacroStore,
) -> SlotManagerCtx<'a> {
    SlotManagerCtx {
        slots,
        slot_triggers,
        sounds,
        macros,
        selected_slot: None,
        configure_available: false,
    }
}

#[test]
fn resolve_slot_returns_empty_for_no_content() {
    let slots = SlotMap::default();
    let sounds: Vec<SoundEntry> = Vec::new();
    let macros = MacroStore::default();
    let triggers = empty_triggers();

    let view = resolve_slot(0, &ctx(&slots, &triggers, &sounds, &macros));

    assert_eq!(view, SlotView::Empty);
}

#[test]
fn resolve_slot_resolves_a_bound_sound() {
    let mut slots = SlotMap::default();
    let s = sound("s1", "/sounds/a.wav");
    slots.set(0, s.path.clone());
    let sounds = vec![s.clone()];
    let macros = MacroStore::default();
    let triggers = empty_triggers();

    let view = resolve_slot(0, &ctx(&slots, &triggers, &sounds, &macros));

    assert_eq!(view, SlotView::Sound(&s));
}

#[test]
fn resolve_slot_resolves_a_bound_macro() {
    let mut slots = SlotMap::default();
    slots.set_macro(1, "m1").expect("valid macro id");
    let sounds: Vec<SoundEntry> = Vec::new();
    let mut macros = MacroStore::default();
    macros.0.push(macro_def("m1", "Honk combo"));
    let triggers = empty_triggers();

    let view = resolve_slot(1, &ctx(&slots, &triggers, &sounds, &macros));

    assert_eq!(view, SlotView::Macro(&macros.0[0]));
}

/// A slot pointing at a sound path no longer in the library (file deleted or
/// moved) must render as `Empty` — and resolution itself must never touch
/// the underlying slot content. Self-clearing that reference is an
/// activation-time concern only (`crate::app::slots`), never a render-time
/// side effect.
#[test]
fn resolve_slot_dangling_sound_is_empty_and_never_self_clears() {
    let mut slots = SlotMap::default();
    slots.set(3, PathBuf::from("/gone/deleted.wav"));
    let sounds: Vec<SoundEntry> = Vec::new();
    let macros = MacroStore::default();
    let triggers = empty_triggers();

    let view = resolve_slot(3, &ctx(&slots, &triggers, &sounds, &macros));

    assert_eq!(view, SlotView::Empty);
    assert!(
        slots.content(3).is_some(),
        "resolve_slot must never self-clear a dangling reference"
    );
}

/// Macro counterpart of the above: an unknown/removed macro id also
/// degrades to `Empty` without mutating the slot.
#[test]
fn resolve_slot_dangling_macro_is_empty_and_never_self_clears() {
    let mut slots = SlotMap::default();
    slots.set_macro(4, "missing-macro").expect("valid macro id");
    let sounds: Vec<SoundEntry> = Vec::new();
    let macros = MacroStore::default();
    let triggers = empty_triggers();

    let view = resolve_slot(4, &ctx(&slots, &triggers, &sounds, &macros));

    assert_eq!(view, SlotView::Empty);
    assert!(
        slots.content(4).is_some(),
        "resolve_slot must never self-clear a dangling reference"
    );
}

#[test]
fn resolve_slot_is_pure_repeated_calls_agree() {
    let mut slots = SlotMap::default();
    let s = sound("s1", "/sounds/a.wav");
    slots.set(0, s.path.clone());
    let sounds = vec![s.clone()];
    let macros = MacroStore::default();
    let triggers = empty_triggers();
    let built = ctx(&slots, &triggers, &sounds, &macros);

    let first = resolve_slot(0, &built);
    let second = resolve_slot(0, &built);

    assert_eq!(first, second);
}

#[test]
fn bound_count_counts_sound_and_macro_slots() {
    let mut slots = SlotMap::default();
    slots.set(0, PathBuf::from("/sounds/a.wav"));
    slots.set_macro(1, "m1").expect("valid macro id");
    // Slot 2 left unassigned.

    assert_eq!(bound_count(&slots), 2);
}

/// Regression pin for the pre-#169 bug: `SlotMap::get` only sees sound
/// content, so counting via `slots.get(i).is_some()` silently undercounts
/// macro-only slots.
#[test]
fn bound_count_matches_content_not_get_for_macro_only_slots() {
    let mut slots = SlotMap::default();
    slots.set_macro(5, "m1").expect("valid macro id");

    assert_eq!(bound_count(&slots), 1);
    assert!(
        slots.get(5).is_none(),
        "sanity check: SlotMap::get only sees sound content"
    );
}
