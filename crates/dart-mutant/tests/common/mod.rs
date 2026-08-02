//! Shared helpers for dart-mutant integration tests.
//!
//! Centralises the binary path resolution, fixture path resolution, and the
//! JSON-parse helper so every test file invokes the tool the same way.
//!
//! ## Contract
//!
//! - Binary under test: `CARGO_BIN_EXE_<bin>` (set by Cargo for integration
//!   tests in a binary crate).
//! - Fixtures live at `{CARGO_MANIFEST_DIR}/../../tests/fixtures/<name>` —
//!   i.e. the repo-root `tests/fixtures/` tree created on the `wf-fixtures`
//!   branch. They are NOT present on this branch, so tests are compiled but
//!   not run here; the parent agent merges fixtures first and runs the suite.
//! - Agent JSON contract (added on `wf-schema`): the `--format json` stdout
//!   gains `"schemaVersion": "1.0"` as its first field. All other fields are
//!   unchanged.
//!
//! Every integration test file compiles this module into its own test
//! binary, and each file only uses a subset of the helpers — hence the
//! module-level dead-code allowance (this is the standard pattern for shared
//! test support modules).

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

/// Absolute path to the dart-mutant binary built for this test run.
///
/// Cargo sets `CARGO_BIN_EXE_<bin-target>` for integration tests, with any
/// hyphens in the target name replaced by underscores — the target is named
/// `dart_mutant`, so the canonical variable is `CARGO_BIN_EXE_dart_mutant`.
/// We accept both spellings so the tests keep working if the target is ever
/// renamed to `dart-mutant`.
pub fn bin() -> PathBuf {
    let exe = option_env!("CARGO_BIN_EXE_dart_mutant")
        .or(option_env!("CARGO_BIN_EXE_dart-mutant"))
        .expect("CARGO_BIN_EXE_dart_mutant not set — integration tests must run via `cargo test`");
    PathBuf::from(exe)
}

/// Absolute path to a named fixture directory under the repo-root
/// `tests/fixtures/` tree.
///
/// `name` is one of `"small"`, `"medium"`, `"large"`.
pub fn fixture_dir(name: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/dart-mutant → ../../tests/fixtures/<name>
    manifest
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Run dart-mutant against `fixture` with the given extra args.
///
/// Always passes `--path <fixture>` as the first argument after the binary
/// path, then appends `extra_args`. Returns the completed child process
/// (stdout captured as `String`, exit code available via `.status.code()`).
///
/// Panics with a clear message if the binary cannot be spawned.
pub fn run_mutant(fixture: &str, extra_args: &[&str]) -> (std::process::Output, String) {
    let path = fixture_dir(fixture);
    let mut cmd = Command::new(bin());
    cmd.arg("--path").arg(&path);
    for a in extra_args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn dart-mutant binary: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

/// Run dart-mutant with a literal `--path` value (used for the nonexistent-path
/// exit-code test where we deliberately pass a bad path).
pub fn run_mutant_raw(path_arg: &str, extra_args: &[&str]) -> (std::process::Output, String) {
    let mut cmd = Command::new(bin());
    cmd.arg("--path").arg(path_arg);
    for a in extra_args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn dart-mutant binary: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

/// Run dart-mutant against a named fixture with extra env vars, inheriting
/// the parent environment (so the tool's child `dart test` processes see
/// them too).
pub fn run_mutant_with_env(
    fixture: &str,
    extra_args: &[&str],
    env: &[(&str, &str)],
) -> (std::process::Output, String) {
    let path = fixture_dir(fixture);
    let mut cmd = Command::new(bin());
    cmd.arg("--path").arg(&path);
    for a in extra_args {
        cmd.arg(a);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn dart-mutant binary: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

/// Parse the full stdout as a single JSON document, returning a
/// `serde_json::Value`. Use `expect` with a context message so failures
/// name the fixture and show a stdout prefix.
pub fn parse_json_stdout(stdout: &str, context: &str) -> serde_json::Value {
    let trimmed = stdout.trim();
    serde_json::from_str(trimmed).unwrap_or_else(|e| {
        panic!(
            "{context}: stdout is not valid JSON ({e}); first 200 bytes: {:?}",
            &trimmed[..trimmed.len().min(200)]
        )
    })
}

/// Assert that the JSON object has the agent-contract `schemaVersion` field
/// equal to `"1.0"` (added on `wf-schema`). Returns the parsed value for
/// further assertions.
pub fn parse_and_check_schema(stdout: &str, context: &str) -> serde_json::Value {
    let v = parse_json_stdout(stdout, context);
    assert_eq!(
        v["schemaVersion"], "1.0",
        "{context}: expected schemaVersion == \"1.0\" (agent JSON contract)"
    );
    v
}
