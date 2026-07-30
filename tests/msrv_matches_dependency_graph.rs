//! Pins the boundary invariant behind issue #218: the declared MSRV in
//! `Cargo.toml` (`package.rust-version`) must always equal the highest
//! `rust_version` declared by any production dependency reachable in the
//! resolved graph. Transitive crates count: Cargo enforces `rust_version`
//! against every package it compiles, not just the ones our own manifest
//! names, so a dependency-of-a-dependency can set the real floor. That is
//! the case today -- `fundsp` pulls in `wide`/`safe_arch`/`resampler` and
//! `lofty` pulls in `ogg_pager`, all of which require 1.89, well above what
//! any direct dependency declares for itself.
//!
//! If some future bump raises the floor further, this test fails loudly
//! instead of letting the crate keep advertising a floor it can no longer
//! build on.
//!
//! The graph is resolved for the host platform only, so the suite still runs
//! inside offline distro package builds (see `metadata_args`).

use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
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

/// Runs `cargo metadata` and returns the declared `rust_version` of every
/// compiled (non-dev) dependency reachable from the root, skipping
/// dependencies that don't declare one at all.
fn compiled_dependency_versions() -> Vec<Version> {
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

    reachable_compiled_packages(root_id, nodes)
        .into_iter()
        .filter_map(|pkg_id| rust_version_of(pkg_id, packages))
        .collect()
}

/// Breadth-first walk of `resolve.nodes` from the root across every edge
/// Cargo compiles. Stopping at the root's own edges would miss the crates
/// that actually bind the floor, since a direct dependency can declare a low
/// `rust_version` for itself while depending on something far newer.
fn reachable_compiled_packages<'a>(root_id: &'a str, nodes: &'a [Value]) -> Vec<&'a str> {
    let by_id: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|node| node["id"].as_str().map(|id| (id, node)))
        .collect();

    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::from([root_id]);
    let mut reachable = Vec::new();

    while let Some(id) = queue.pop_front() {
        let Some(node) = by_id.get(id) else { continue };
        for dep in node["deps"].as_array().into_iter().flatten() {
            let Some(pkg) = dep["pkg"].as_str() else {
                continue;
            };
            if is_compiled_dependency(dep) && seen.insert(pkg) {
                reachable.push(pkg);
                queue.push_back(pkg);
            }
        }
    }
    reachable
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Asks the same cargo that is driving this test which triple it builds for,
/// so the metadata filter tracks the machine actually running the suite.
fn host_triple(cargo: &str) -> String {
    let output = Command::new(cargo)
        .arg("-vV")
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} -vV`: {e}"));
    assert!(
        output.status.success(),
        "`{cargo} -vV` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or_else(|| panic!("`{cargo} -vV` reported no host triple"))
        .trim()
        .to_string()
}

/// Cargo resolves for every platform by default, which makes it fetch crates
/// that only exist for foreign targets (Android's `android-activity`, the
/// Apple and Windows winit backends). Distro packagers prime their cache with
/// `cargo fetch --target <host>` and then build with no network, so an
/// unfiltered query aborts there. Pin the host triple to resolve exactly the
/// subset the packager fetched.
fn metadata_args(host: &str) -> Vec<String> {
    ["metadata", "--format-version", "1", "--offline"]
        .iter()
        .map(|arg| (*arg).to_string())
        .chain(["--filter-platform".to_string(), host.to_string()])
        .collect()
}

fn run_cargo_metadata() -> Value {
    let cargo = cargo_bin();
    let output = Command::new(&cargo)
        .args(metadata_args(&host_triple(&cargo)))
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

/// An edge counts when Cargo has to compile the crate on the other end of it
/// for a plain `cargo build`: normal dependencies (`null` kind, cargo's JSON
/// encoding for the default) and build dependencies, whose build scripts go
/// through the same rustc and are held to the same floor. Dev-dependencies
/// are excluded -- a consumer of this crate never builds them.
fn is_compiled_dependency(dep: &Value) -> bool {
    dep["dep_kinds"].as_array().is_some_and(|kinds| {
        kinds
            .iter()
            .any(|kind| kind["kind"].is_null() || kind["kind"] == "build")
    })
}

fn rust_version_of(pkg_id: &str, packages: &[Value]) -> Option<Version> {
    packages.iter().find(|pkg| pkg["id"] == pkg_id)?["rust_version"]
        .as_str()
        .map(parse_version)
}

/// Equality, not `>=`, on purpose. `>=` alone would let the declared floor
/// drift arbitrarily high and lock out users for no reason, and an inflated
/// MSRV is invisible until someone tries the toolchain it excludes. The one
/// case equality gets wrong is our own source requiring a newer compiler than
/// every dependency does; the failure message names that case so nobody
/// "fixes" a red test by declaring a floor the crate cannot build on.
#[test]
fn msrv_equals_max_compiled_dependency_rust_version() {
    let declared = declared_rust_version();
    let deps = compiled_dependency_versions();
    let max_dep = deps
        .into_iter()
        .max()
        .expect("at least one compiled dependency must declare rust_version");

    assert_eq!(
        declared, max_dep,
        "package.rust-version ({declared:?}) must equal the highest rust_version \
         among compiled dependencies, transitive ones included ({max_dep:?}). \
         If {max_dep:?} is the real floor, update Cargo.toml to match. If our own \
         source now needs a newer compiler than any dependency, raise \
         rust-version to what the source needs and relax this assertion to \
         `declared >= max_dep` -- never lower it below the source requirement"
    );
}

/// Regression guard: the AUR `honkhonk-git` build primes its cache with
/// `cargo fetch --target <host>` and then runs `check()` with no network, so
/// dropping the platform filter breaks packaging even though every developer
/// machine — which has the foreign-target crates cached already — stays green.
#[test]
fn metadata_query_is_offline_and_pinned_to_the_host_platform() {
    let host = host_triple(&cargo_bin());
    assert!(host.contains('-'), "expected a target triple, got {host:?}");

    let args = metadata_args(&host);
    assert!(
        args.iter().any(|arg| arg == "--offline"),
        "the query must not reach the network: {args:?}"
    );
    let filter = args
        .iter()
        .position(|arg| arg == "--filter-platform")
        .expect("the query must resolve one platform, not every platform");
    assert_eq!(args.get(filter + 1), Some(&host));
}
