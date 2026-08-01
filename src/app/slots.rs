//! Shortcut-slot activation and macro-slot assignment (app layer, #169).
//! Resolves a slot's content ([`crate::state::SlotContent`]) at press time and
//! dispatches to the right play path, self-clearing any reference that no
//! longer resolves (a deleted sound file or a removed macro). Mirrors the
//! shipped [`super::macros`] controller: pure `impl HonkHonk` methods, no new
//! state-layer struct.

use std::path::PathBuf;

use iced::Task;

use super::{HonkHonk, Message};
use crate::state::SlotContent;

mod rows;
#[cfg(test)]
mod sort_tests;

// Test-only spies for `HonkHonk::persist_slots`: populated unconditionally,
// ahead of the `self.persist` gate, so a test can prove a slot mutation
// actually reached the persist call. `HonkHonk::new_for_test()` hardcodes
// `persist: false` (see `mod.rs`) so `cargo test` never touches the real
// XDG config dir — which also makes the real disk write a guaranteed no-op
// and leaves the call itself unobservable without these spies (#169 review).
//
// Thread-local, not a process-wide static: `cargo test`'s worker threads run
// one `#[test]` fn to completion before picking up the next, so within a
// single test's execution only that test's own code can touch its thread's
// cell — a call made by an unrelated test running concurrently on another
// thread can never leak into this test's before/after delta. A process-wide
// counter could not make that attribution guarantee.
//
// Compiled only under `cfg(test)`; zero footprint on release builds.
#[cfg(test)]
thread_local! {
    /// Count of `persist_slots` calls observed on this thread.
    static PERSIST_SLOTS_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Clone of `self.slots` taken *inside* `persist_slots`, at the instant
    /// it runs — lets a test prove ordering (the mutation was already
    /// applied before persistence ran), not just that persistence ran at
    /// some point.
    static PERSIST_SLOTS_SNAPSHOT: std::cell::RefCell<Option<crate::state::SlotMap>> =
        const { std::cell::RefCell::new(None) };
}

/// Current value of this thread's [`PERSIST_SLOTS_CALLS`] cell. Assertions
/// must compare a before/after delta (`after > before`), never an absolute
/// value — see the thread-local's doc comment for why the delta is safe to
/// trust even under parallel test execution.
#[cfg(test)]
pub(crate) fn persist_slots_call_count() -> u32 {
    PERSIST_SLOTS_CALLS.with(std::cell::Cell::get)
}

/// The slot map as it looked *inside* the most recent [`HonkHonk::persist_slots`]
/// call on this thread, or `None` if it has not been called yet this test.
/// Lets a test assert that a specific mutation was already visible at
/// persist-time, pinning ordering rather than mere occurrence.
#[cfg(test)]
pub(crate) fn persist_slots_last_snapshot() -> Option<crate::state::SlotMap> {
    PERSIST_SLOTS_SNAPSHOT.with(|snapshot| snapshot.borrow().clone())
}

impl HonkHonk {
    /// A shortcut (or the slot manager) fired slot `idx`. Resolves the slot's
    /// content and dispatches: a missing slot is a no-op, a sound plays via
    /// `request_play`, a macro plays via `play_macro`. Never panics — an
    /// out-of-range `idx` is already a safe no-op in [`SlotMap::content`].
    pub(crate) fn activate_slot(&mut self, idx: u8) -> Task<Message> {
        match self.slots.content(idx).cloned() {
            None => Task::none(),
            Some(SlotContent::Sound(path)) => self.activate_sound_slot(idx, path),
            Some(SlotContent::Macro(macro_id)) => self.activate_macro_slot(idx, macro_id),
        }
    }

    /// Plays the library sound at `path` if it still exists; otherwise the
    /// slot outlived its target (file deleted/moved) and is cleared.
    fn activate_sound_slot(&mut self, idx: u8, path: PathBuf) -> Task<Message> {
        if let Some(sound) = self.sounds.iter().find(|s| s.path == path).cloned() {
            return self.request_play(&sound, true);
        }
        tracing::warn!(
            slot = idx + 1,
            ?path,
            "slot points to missing file; clearing stale slot"
        );
        self.clear_stale_slot(idx);
        Task::none()
    }

    /// Fires `macro_id` unconditionally once it is known to exist — including
    /// an existing-but-zero-step macro, a valid authoring state that must
    /// never self-clear. An unknown/deleted id is the only stale case here,
    /// and clears the slot without ever reaching `play_macro`.
    fn activate_macro_slot(&mut self, idx: u8, macro_id: String) -> Task<Message> {
        if self.macros.get(&macro_id).is_none() {
            tracing::warn!(
                slot = idx + 1,
                macro_id = %macro_id,
                "slot points to missing macro; clearing stale slot"
            );
            self.clear_stale_slot(idx);
            return Task::none();
        }
        self.play_macro(&macro_id)
    }

    /// Shared self-clear: drops the slot's content and persists the change
    /// under the same switch as every other slot mutation.
    fn clear_stale_slot(&mut self, idx: u8) {
        self.slots.clear(idx);
        self.persist_slots();
    }

    /// Binds slot `idx` to macro `macro_id`. Mutates and persists only if the
    /// id passes `SlotMap::set_macro`'s validation; a rejected id leaves the
    /// slot's existing content untouched.
    pub(crate) fn assign_macro_slot(&mut self, idx: u8, macro_id: String) -> Task<Message> {
        match self.slots.set_macro(idx, macro_id) {
            Ok(()) => self.persist_slots(),
            Err(e) => {
                tracing::warn!(slot = idx + 1, error = %e, "macro slot assignment rejected");
            }
        }
        Task::none()
    }

    /// Persists the slot map under the same persistence switch as the config.
    /// Colocated here with its two slot-mutation call sites above
    /// (`clear_stale_slot`, `assign_macro_slot`); also called from
    /// `mod.rs`'s `AssignSlot`/`ClearSlot` message arms.
    pub(super) fn persist_slots(&self) {
        #[cfg(test)]
        {
            PERSIST_SLOTS_CALLS.with(|calls| calls.set(calls.get() + 1));
            PERSIST_SLOTS_SNAPSHOT
                .with(|snapshot| *snapshot.borrow_mut() = Some(self.slots.clone()));
        }
        if self.persist
            && let Err(e) = self.slots.save()
        {
            tracing::warn!(error = %e, "slots save error");
        }
    }
}

#[cfg(test)]
mod tests;
