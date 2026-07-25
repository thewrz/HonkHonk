//! Fixed shortcut-slot assignments and their persistence contract.

use std::path::{Path, PathBuf};

use thiserror::Error;

mod persistence;

const SLOTS_FILE_NAME: &str = "slots.json";
const CONFIG_DIR_NAME: &str = "honkhonk";
const SLOT_COUNT: usize = 20;
const MAX_MACRO_ID_BYTES: usize = 255;

/// The state-layer content assigned to one shortcut slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotContent {
    Sound(PathBuf),
    Macro(String),
}

impl SlotContent {
    fn macro_with_id(id: String) -> Result<Self, MacroIdError> {
        validate_macro_id(&id)?;
        Ok(Self::Macro(id))
    }
}

/// Why a macro ID cannot be stored in a slot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MacroIdError {
    #[error("macro ID cannot be empty")]
    Empty,
    #[error("macro ID is {length} bytes; the maximum is {max}")]
    TooLong { length: usize, max: usize },
    #[error("macro ID cannot contain control characters")]
    ControlCharacter,
}

fn validate_macro_id(id: &str) -> Result<(), MacroIdError> {
    if id.is_empty() {
        return Err(MacroIdError::Empty);
    }
    if id.len() > MAX_MACRO_ID_BYTES {
        return Err(MacroIdError::TooLong {
            length: id.len(),
            max: MAX_MACRO_ID_BYTES,
        });
    }
    if id.chars().any(char::is_control) {
        return Err(MacroIdError::ControlCharacter);
    }
    Ok(())
}

/// The fixed 20-slot assignment map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotMap {
    slots: [Option<SlotContent>; SLOT_COUNT],
    writable: bool,
}

impl Default for SlotMap {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            writable: true,
        }
    }
}

impl SlotMap {
    fn read_protected() -> Self {
        Self {
            writable: false,
            ..Self::default()
        }
    }

    /// Returns any content assigned to `idx`.
    pub fn content(&self, idx: u8) -> Option<&SlotContent> {
        self.slots.get(idx as usize)?.as_ref()
    }

    /// Returns the sound at `idx`, excluding macro assignments.
    pub fn get(&self, idx: u8) -> Option<&PathBuf> {
        match self.content(idx)? {
            SlotContent::Sound(path) => Some(path),
            SlotContent::Macro(_) => None,
        }
    }

    /// Assigns a sound path. An out-of-range index is a no-op.
    pub fn set(&mut self, idx: u8, path: PathBuf) {
        if let Some(slot) = self.slots.get_mut(idx as usize) {
            *slot = Some(SlotContent::Sound(path));
        }
    }

    /// Assigns a validated macro ID. An out-of-range index is a no-op.
    pub fn set_macro(&mut self, idx: u8, id: impl Into<String>) -> Result<(), MacroIdError> {
        let Some(slot) = self.slots.get_mut(idx as usize) else {
            return Ok(());
        };
        let content = SlotContent::macro_with_id(id.into())?;
        *slot = Some(content);
        Ok(())
    }

    /// Returns the macro ID at `idx`, excluding sound assignments.
    pub fn macro_id(&self, idx: u8) -> Option<&str> {
        match self.content(idx)? {
            SlotContent::Macro(id) => Some(id),
            SlotContent::Sound(_) => None,
        }
    }

    /// Clears either content kind. An out-of-range index is a no-op.
    pub fn clear(&mut self, idx: u8) {
        if let Some(slot) = self.slots.get_mut(idx as usize) {
            *slot = None;
        }
    }

    /// Returns the first slot assigned to `path`, excluding macro IDs.
    pub fn slot_for(&self, path: &Path) -> Option<u8> {
        self.slots
            .iter()
            .position(|slot| matches!(slot, Some(SlotContent::Sound(value)) if value == path))
            .map(|index| index as u8)
    }

    /// Returns the first slot assigned to `id`, excluding sound paths.
    pub fn slot_for_macro(&self, id: &str) -> Option<u8> {
        self.slots
            .iter()
            .position(|slot| matches!(slot, Some(SlotContent::Macro(value)) if value == id))
            .map(|index| index as u8)
    }
}

#[cfg(test)]
mod tests;
