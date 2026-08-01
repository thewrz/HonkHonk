//! Pins the boundary invariants behind issue #226's `msrv` CI job in
//! `.github/workflows/rust.yml`. Unlike `msrv_matches_dependency_graph.rs`
//! (which checks the declared floor against the *dependency graph*), this
//! suite checks the *workflow file itself*: that the job actually builds
//! against the version Cargo.toml declares, fails loudly instead of
//! silently degrading, and never disturbs the job branch protection keys
//! off of or the checked-in lockfile.
//!
//! The workflow is parsed as text rather than through a YAML crate --
//! matching the existing `Cargo.toml`-parsing convention in this suite --
//! to avoid a new dependency for a handful of scalar/step lookups.

use std::path::Path;
use std::process::{Command, Output};

fn read_workflow() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.github/workflows/rust.yml");
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// True for a top-level (2-space indent) `key:` line, i.e. a job header --
/// as opposed to a step field (deeper indent) or a `  # comment` line.
fn is_job_header(line: &str) -> bool {
    let two_space_indent = line.starts_with("  ") && !line.starts_with("   ");
    let trimmed = line.trim_start();
    two_space_indent && !trimmed.starts_with('#') && !trimmed.is_empty() && trimmed.ends_with(':')
}

/// Slices out one job's lines (its header through the line before the next
/// job header), preserving original indentation.
fn job_block_lines<'a>(lines: &[&'a str], job_name: &str) -> Vec<&'a str> {
    let header = format!("  {job_name}:");
    let start = lines
        .iter()
        .position(|line| *line == header)
        .unwrap_or_else(|| panic!("job `{job_name}` not found in rust.yml"));
    let end = lines[(start + 1)..]
        .iter()
        .position(|line| is_job_header(line))
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    lines[start..end].to_vec()
}

/// Finds a step by its `- name:` line and returns the index range
/// `[step_start, next_step_start)` within `block`.
fn step_range(block: &[&str], step_name: &str) -> (usize, usize) {
    let marker = format!("- name: {step_name}");
    let start = block
        .iter()
        .position(|line| line.trim_start() == marker)
        .unwrap_or_else(|| panic!("step `{step_name}` not found"));
    let end = block[(start + 1)..]
        .iter()
        .position(|line| {
            let t = line.trim_start();
            t.starts_with("- name:") || t.starts_with("- uses:")
        })
        .map(|offset| start + 1 + offset)
        .unwrap_or(block.len());
    (start, end)
}

fn toolchain_value_for_step<'a>(block: &[&'a str], step_name: &str) -> &'a str {
    let (start, end) = step_range(block, step_name);
    block[start..end]
        .iter()
        .find_map(|line| line.trim_start().strip_prefix("toolchain:"))
        .map(str::trim)
        .unwrap_or_else(|| panic!("step `{step_name}` has no toolchain: field"))
}

fn run_command_for_step<'a>(block: &[&'a str], step_name: &str) -> &'a str {
    let (start, end) = step_range(block, step_name);
    block[start..end]
        .iter()
        .find_map(|line| line.trim_start().strip_prefix("run:"))
        .map(str::trim)
        .unwrap_or_else(|| panic!("step `{step_name}` has no single-line `run:` command"))
}

/// Extracts the literal shell script under a `run: |` block scalar, keeping
/// each line's original indentation (harmless for plain sequential bash).
fn extract_run_block(block: &[&str], step_name: &str) -> String {
    let (start, end) = step_range(block, step_name);
    let run_idx = block[start..end]
        .iter()
        .position(|line| line.trim_start() == "run: |")
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("step `{step_name}` has no `run: |` block"));
    let run_indent = block[run_idx].len() - block[run_idx].trim_start().len();
    block[(run_idx + 1)..end]
        .iter()
        .take_while(|line| {
            line.trim().is_empty() || (line.len() - line.trim_start().len()) > run_indent
        })
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_fixture_cargo_toml(dir: &Path, rust_version_line: Option<&str>) {
    let mut contents = String::from("[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n");
    if let Some(line) = rust_version_line {
        contents.push_str(line);
        contents.push('\n');
    }
    std::fs::write(dir.join("Cargo.toml"), contents).expect("failed to write fixture Cargo.toml");
}

fn run_extracted_script(script: &str, cwd: &Path, github_output: &Path) -> Output {
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(cwd)
        .env("GITHUB_OUTPUT", github_output)
        .output()
        .expect("failed to spawn bash for the extracted read-msrv script")
}

fn output_has_version(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|contents| contents.contains("version="))
        .unwrap_or(false)
}

/// True when `dir` sits inside a git working tree. Governs whether the
/// Cargo.toml-drift check below has anything to diff against -- a tarball
/// checkout (e.g. an AUR `source=()` archive extracted via `git archive`,
/// which never writes a `.git` directory) is not a git repo at all, so
/// `git diff` there fails with "fatal: not a git repository" rather than
/// reporting a clean/dirty status.
fn is_inside_git_work_tree(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Invariant: the msrv job's toolchain is always derived from Cargo.toml's
/// `rust-version` at run time, never hardcoded in the workflow.
#[test]
fn msrv_toolchain_is_derived_from_cargo_toml_not_hardcoded() {
    let text = read_workflow();
    let lines: Vec<&str> = text.lines().collect();
    let block = job_block_lines(&lines, "msrv");

    let toolchain = toolchain_value_for_step(&block, "Install Rust toolchain (MSRV)");

    assert_eq!(
        toolchain, "${{ steps.read-msrv.outputs.version }}",
        "msrv job must install the toolchain named by read-msrv's output, not a literal version"
    );
}

/// Invariant: the read-msrv step never lets a missing or malformed
/// `rust-version` line silently degrade to an empty/default toolchain
/// input -- it must fail the job loudly instead.
#[test]
fn read_msrv_step_fails_loudly_on_missing_or_malformed_version() {
    let text = read_workflow();
    let lines: Vec<&str> = text.lines().collect();
    let block = job_block_lines(&lines, "msrv");
    let script = extract_run_block(&block, "Read declared MSRV from Cargo.toml");
    assert!(!script.is_empty(), "read-msrv script must not be empty");

    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let output_path = tmp.path().join("github_output");

    write_fixture_cargo_toml(tmp.path(), None);
    let missing = run_extracted_script(&script, tmp.path(), &output_path);
    assert!(
        !missing.status.success(),
        "must exit non-zero when rust-version is absent"
    );
    assert!(
        !output_has_version(&output_path),
        "must never emit a version output when rust-version is absent"
    );

    let _ = std::fs::remove_file(&output_path);
    write_fixture_cargo_toml(tmp.path(), Some("rust-version = \"not-a-version\""));
    let malformed = run_extracted_script(&script, tmp.path(), &output_path);
    assert!(
        !malformed.status.success(),
        "must exit non-zero on a malformed rust-version"
    );
    assert!(
        !output_has_version(&output_path),
        "must never emit a version output for a malformed rust-version"
    );

    let _ = std::fs::remove_file(&output_path);
    write_fixture_cargo_toml(tmp.path(), Some("rust-version = \"1.89\""));
    let ok = run_extracted_script(&script, tmp.path(), &output_path);
    assert!(
        ok.status.success(),
        "a well-formed rust-version must succeed"
    );
    let emitted =
        std::fs::read_to_string(&output_path).expect("GITHUB_OUTPUT must exist on success");
    assert_eq!(emitted.trim(), "version=1.89");
}

/// Invariant: the existing `build` job is untouched, so branch protection's
/// required status check (which matches on job id and/or display name)
/// keeps matching.
#[test]
fn build_job_identity_is_untouched() {
    let text = read_workflow();
    let lines: Vec<&str> = text.lines().collect();
    let block = job_block_lines(&lines, "build");

    assert_eq!(
        block[0], "  build:",
        "the `build` job id must not be renamed"
    );
    assert_eq!(
        block.get(1).copied(),
        Some("    name: Build (release)"),
        "the `build` job's display name must not change, or branch protection stops matching"
    );
}

/// Invariant: `cargo check --locked` in the msrv job must never mutate
/// `Cargo.lock`. Checks that the workflow still names exactly that command,
/// and that actually running it succeeds. A separate before/after byte
/// comparison of `Cargo.lock` is deliberately not done here: `--locked`
/// makes cargo itself refuse to write the lockfile and instead exit
/// non-zero on any drift, so that comparison could never fail independently
/// of the `status.success()` assertion below -- it would only ever
/// duplicate what this test already checks.
#[test]
fn msrv_cargo_check_is_locked_and_never_mutates_cargo_lock() {
    let text = read_workflow();
    let lines: Vec<&str> = text.lines().collect();
    let block = job_block_lines(&lines, "msrv");
    let command = run_command_for_step(&block, "cargo check (MSRV)");
    assert_eq!(
        command, "cargo check --locked",
        "the msrv job's check step must run exactly `cargo check --locked`"
    );

    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let mut parts = command.split_whitespace();
    let program = parts.next().expect("extracted command must not be empty");
    let status = Command::new(program)
        .args(parts)
        .current_dir(manifest_dir)
        .status()
        .expect("failed to spawn the extracted cargo check command");
    assert!(
        status.success(),
        "cargo check --locked must succeed against an up-to-date Cargo.lock"
    );
}

/// Regression guard: proving the `msrv` job goes red on a violation requires
/// temporarily lowering `Cargo.toml`'s `rust-version` below the toolchain
/// under test, then reverting it (see the issue #226 verification
/// experiment). If that revert is ever skipped, the mutated file would ride
/// along uncommitted into the next commit. A working-tree diff against the
/// git index catches exactly that mistake -- deliberately not a diff against
/// `origin/main`, since CI's checkout step for the `test` job is a shallow,
/// single-ref clone with no `main`/`origin/main` ref to resolve, which would
/// make that comparison error out rather than assert anything.
///
/// This check only applies inside an actual git checkout. A tarball build
/// (e.g. the AUR `honkhonk` package, whose `PKGBUILD` downloads a
/// `git archive` tag tarball with no `.git` directory and runs
/// `cargo test --frozen --release` against it unconditionally) has nothing
/// to diff against, so it is skipped rather than failed there -- see
/// `cargo_toml_drift_check_is_a_no_op_outside_a_git_checkout` below for the
/// reproduction this guards against.
#[test]
fn cargo_toml_has_no_uncommitted_drift_from_the_msrv_verification_experiment() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = Path::new(manifest_dir);

    if !is_inside_git_work_tree(manifest_path) {
        return;
    }

    let output = Command::new("git")
        .args(["diff", "--exit-code", "--", "Cargo.toml"])
        .current_dir(manifest_dir)
        .output()
        .expect("failed to spawn git diff");

    assert!(
        output.status.success(),
        "Cargo.toml has uncommitted changes -- if this is left over from the \
         msrv job's verification experiment (issue #226), revert it before \
         committing:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// Reproduces the AUR `honkhonk` package build tree: a plain directory with
/// no `.git` at all, exactly what `git archive HEAD | tar -x` (what the
/// `PKGBUILD`'s `source=()` tag tarball ultimately contains) produces.
/// Before the `is_inside_git_work_tree` guard, `git diff --exit-code` in
/// that directory exits non-zero with "fatal: not a git repository" and the
/// drift check misreports it as "Cargo.toml has uncommitted changes".
#[test]
fn cargo_toml_drift_check_is_a_no_op_outside_a_git_checkout() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    write_fixture_cargo_toml(tmp.path(), Some("rust-version = \"1.89\""));

    assert!(
        !is_inside_git_work_tree(tmp.path()),
        "a bare tarball checkout must not be mistaken for a git work tree"
    );

    let output = Command::new("git")
        .args(["diff", "--exit-code", "--", "Cargo.toml"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to spawn git diff");
    assert!(
        !output.status.success(),
        "git diff itself is expected to fail outside a git repo -- this is exactly \
         the failure the is_inside_git_work_tree guard must prevent from reaching \
         the drift assertion"
    );
}
