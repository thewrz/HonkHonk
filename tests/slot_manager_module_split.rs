//! Pins the file-size invariant (CLAUDE.md "Project Overrides": 400-line
//! cap) for the `src/ui/slot_manager` directory module and its app-layer
//! counterpart `src/app/slots.rs` (#169).
//!
//! Exact file names/count under `src/ui/slot_manager` are deliberately NOT
//! pinned here: that is internal file organization, not observable
//! behavior, and a compatible rename/merge (e.g. `macro_slot.rs` ->
//! `macro_tile.rs`) would fail such a test for no user-visible reason. Code
//! review and the file-size cap below already enforce the split (#169
//! review).
//!
//! `cargo build` succeeding is pinned implicitly: this test binary cannot
//! compile at all — let alone run — unless the crate (including the split
//! module) builds cleanly.

use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINES: usize = 400;

fn slot_manager_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/slot_manager")
}

fn slots_module_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/slots.rs")
}

fn assert_under_line_cap(path: &Path) {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let line_count = contents.lines().count();
    assert!(
        line_count <= MAX_LINES,
        "{} has {line_count} lines, exceeds the {MAX_LINES}-line project cap (CLAUDE.md)",
        path.display()
    );
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
        assert_under_line_cap(&path);
    }
}

#[test]
fn slots_module_stays_under_the_line_cap() {
    assert_under_line_cap(&slots_module_path());
}
