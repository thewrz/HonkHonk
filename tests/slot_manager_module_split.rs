//! Pins the structural invariants for the `src/ui/slot_manager` directory-module
//! split (#169 Task 2 mechanical split, extended by Task 3's content-kind file
//! for macro slots).
//!
//! Two invariants are pinned here:
//! 1. `slot_manager` is a directory module split by content-kind into the files
//!    listed in `EXPECTED_FILES` (mod.rs, sound.rs, empty.rs, macro_slot.rs).
//! 2. Every `.rs` file under that directory stays within the project's
//!    400-line file cap (see CLAUDE.md "Project Overrides").
//!
//! `cargo build` succeeding is pinned implicitly: this test binary cannot
//! compile at all — let alone run — unless the crate (including the split
//! module) builds cleanly.

use std::fs;
use std::path::Path;

const MAX_LINES: usize = 400;
const EXPECTED_FILES: &[&str] = &["mod.rs", "sound.rs", "empty.rs", "macro_slot.rs"];

fn slot_manager_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/slot_manager")
}

#[test]
fn slot_manager_is_split_into_expected_files() {
    let dir = slot_manager_dir();
    assert!(
        dir.is_dir(),
        "expected src/ui/slot_manager to be a directory module, found: {}",
        dir.display()
    );
    for name in EXPECTED_FILES {
        let path = dir.join(name);
        assert!(
            path.is_file(),
            "expected src/ui/slot_manager/{name} to exist after the mechanical split"
        );
    }
}

#[test]
fn slot_manager_files_stay_under_the_line_cap() {
    let dir = slot_manager_dir();
    let entries =
        fs::read_dir(&dir).unwrap_or_else(|e| panic!("failed to read dir {}: {e}", dir.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let line_count = contents.lines().count();
        assert!(
            line_count <= MAX_LINES,
            "{} has {line_count} lines, exceeds the {MAX_LINES}-line project cap (CLAUDE.md)",
            path.display()
        );
    }
}
