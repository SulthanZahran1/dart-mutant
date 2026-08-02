//! Contract tests — the agent-facing JSON contract and the `--mutant`
//! re-run contract.
//!
//! ## Agent JSON contract
//!
//! The `--format json --quiet --no-color` stdout is a single JSON document:
//! the `PipelineSummary` with `"schemaVersion": "1.0"` as its first field.
//! The stdout must:
//!   - parse as one whole JSON document (no non-JSON prefix/trailer),
//!   - contain NO ANSI escape bytes (`\x1b`).
//!
//! ## `--mutant` re-run
//!
//! `--mutant <id>` re-runs a single mutant by its numeric ID. The contract:
//! exit 0, JSON output with `total == 1` (or a filtered result).

mod common;

use common::{parse_and_check_schema, run_mutant};

/// `--format json --quiet --no-color` on small: parsed JSON has
/// `schemaVersion == "1.0"`, stdout contains no ANSI escape bytes, and the
/// whole stdout parses as one JSON document (no non-JSON prefix).
#[test]
fn test_agent_json_schema_version() {
    let (output, stdout) = run_mutant("small", &["--format", "json", "--quiet", "--no-color"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "small: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // No ANSI escape bytes anywhere in stdout.
    assert!(
        !stdout.contains('\u{1b}'),
        "agent JSON: stdout must contain no ANSI escape bytes; got: {:?}",
        &stdout[..stdout.len().min(120)]
    );

    // parse_and_check_schema asserts schemaVersion == "1.0" AND that the
    // whole trimmed stdout parses as one JSON document (no non-JSON prefix).
    let _v = parse_and_check_schema(&stdout, "test_agent_json_schema_version");
}

/// `--mutant <id>` re-runs a single mutant: exit 0, JSON output with
/// `total == 1` (or a filtered single-mutant result).
///
/// We first run the full small fixture to discover the first mutant ID, then
/// re-run just that one.
#[test]
fn test_mutant_rerun() {
    // First run: get the full summary so we can read the total mutant count
    // and confirm mutant IDs are 0-indexed integers.
    let (full_output, full_stdout) =
        run_mutant("small", &["--format", "json", "--quiet", "--no-color"]);
    assert_eq!(
        full_output.status.code(),
        Some(0),
        "small: expected exit 0 on full run; stderr: {}",
        String::from_utf8_lossy(&full_output.stderr)
    );
    let full = parse_and_check_schema(&full_stdout, "test_mutant_rerun (full run)");
    let full_total = full["total"]
        .as_u64()
        .expect("small: JSON missing numeric `total`") as usize;
    assert!(
        full_total >= 1,
        "small: need at least one mutant to test --mutant rerun; got {full_total}"
    );

    // Re-run the first mutant (id "0").
    let (rerun_output, rerun_stdout) = run_mutant(
        "small",
        &["--mutant", "0", "--format", "json", "--quiet", "--no-color"],
    );
    assert_eq!(
        rerun_output.status.code(),
        Some(0),
        "small --mutant 0: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&rerun_output.stderr)
    );

    // The filtered run should report total == 1 (single mutant).
    let rerun = parse_and_check_schema(&rerun_stdout, "test_mutant_rerun (--mutant 0)");
    let rerun_total = rerun["total"]
        .as_u64()
        .expect("small --mutant 0: JSON missing numeric `total`") as usize;
    assert_eq!(
        rerun_total, 1,
        "small --mutant 0: expected total == 1, got {rerun_total}"
    );
}
