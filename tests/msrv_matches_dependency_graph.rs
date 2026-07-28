//! Pins the boundary invariant behind issue #218: the declared MSRV in
//! `Cargo.toml` (`package.rust-version`) must always equal the highest
//! `rust_version` declared by any direct, production dependency in the
//! resolved graph. Iced is the binding constraint today; if some future
//! dependency bump raises the real floor further, this test fails loudly
//! instead of letting the crate keep advertising a floor it can no longer
//! build on.

use serde_json::Value;
use std::process::Command;

/// A minimal `major.minor.patch` version -- enough to order the plain
/// numeric strings Cargo emits for `rust_version` (that field never carries
/// pre-release or build metadata).
type Version = (u32, u32, u32);

fn parse_version(raw: &str) -> Version {
    let mut parts = raw.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Reads `package.rust-version` straight out of `Cargo.toml`'s text. Avoids
/// pulling in a TOML-parsing dependency for a single scalar field this test
/// owns exclusively.
fn declared_rust_version() -> Version {
    let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let manifest = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("failed to read {manifest_path}: {e}"));
    let line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("rust-version"))
        .expect("Cargo.toml must declare package.rust-version");
    let value = line
        .split('=')
        .nth(1)
        .and_then(|v| v.split('"').nth(1))
        .unwrap_or_else(|| panic!("malformed rust-version line: {line:?}"));
    parse_version(value)
}

/// Runs `cargo metadata --offline` and returns the declared `rust_version`
/// of every direct, production (non-dev, non-build) dependency, skipping
/// dependencies that don't declare one at all.
fn direct_production_dependency_versions() -> Vec<Version> {
    let metadata = run_cargo_metadata();
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata.packages must be an array");
    let root_id = metadata["resolve"]["root"]
        .as_str()
        .expect("metadata.resolve.root must name the workspace package");
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("metadata.resolve.nodes must be an array");
    let root_node = nodes
        .iter()
        .find(|node| node["id"] == root_id)
        .expect("resolve.nodes must contain the root package");

    root_node["deps"]
        .as_array()
        .expect("root node deps must be an array")
        .iter()
        .filter(|dep| is_normal_dependency(dep))
        .filter_map(|dep| rust_version_of(dep["pkg"].as_str().unwrap_or_default(), packages))
        .collect()
}

fn run_cargo_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(&cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} metadata`: {e}"));
    assert!(
        output.status.success(),
        "`{cargo} metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata must emit valid JSON")
}

/// A dependency is "normal" (production) when at least one of its
/// `dep_kinds` entries has a `null` kind -- cargo's JSON encoding for the
/// default (non-dev, non-build) dependency kind.
fn is_normal_dependency(dep: &Value) -> bool {
    dep["dep_kinds"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"].is_null()))
}

fn rust_version_of(pkg_id: &str, packages: &[Value]) -> Option<Version> {
    packages.iter().find(|pkg| pkg["id"] == pkg_id)?["rust_version"]
        .as_str()
        .map(parse_version)
}

#[test]
fn msrv_equals_max_direct_production_dependency_rust_version() {
    let declared = declared_rust_version();
    let deps = direct_production_dependency_versions();
    let max_dep = deps
        .into_iter()
        .max()
        .expect("at least one direct production dependency must declare rust_version");

    assert_eq!(
        declared, max_dep,
        "package.rust-version ({declared:?}) must equal the highest rust_version \
         among direct production dependencies ({max_dep:?}); update Cargo.toml's \
         rust-version to match"
    );
}
