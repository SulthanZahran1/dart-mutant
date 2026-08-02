//! Harness tests — basic end-to-end runs against the small / medium / large
//! fixtures.
//!
//! These prove the tool actually runs, exits 0, and produces a parseable JSON
//! summary with a sane mutant count. They are the smoke tests that gate every
//! other suite.
//!
//! ## Fixtures
//!
//! Fixtures are created on the `wf-fixtures` branch and merged by the parent
//! before this suite is run. They live at `tests/fixtures/{small,medium,large}`
//! relative to the repo root. On this branch they may be absent — that's
//! expected; we only verify these tests compile.

mod common;

use common::{parse_and_check_schema, run_mutant};

/// Small fixture: exit 0, stdout parses as JSON, `total` >= 10, `killed` >= 10.
#[test]
fn test_harness_small_run() {
    let (output, stdout) = run_mutant("small", &["--format", "json", "--quiet", "--no-color"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "small fixture should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_harness_small_run");
    let total = v["total"]
        .as_u64()
        .expect("small: JSON missing numeric `total`") as usize;
    let killed = v["killed"]
        .as_u64()
        .expect("small: JSON missing numeric `killed`") as usize;

    assert!(
        total >= 10,
        "small: expected total >= 10 mutants, got {total}"
    );
    assert!(killed >= 10, "small: expected killed >= 10, got {killed}");
}

/// Medium fixture: exit 0, `total` in `50..=80`.
#[test]
fn test_harness_medium_run() {
    let (output, stdout) = run_mutant("medium", &["--format", "json", "--quiet", "--no-color"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "medium fixture should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_harness_medium_run");
    let total = v["total"]
        .as_u64()
        .expect("medium: JSON missing numeric `total`") as usize;

    assert!(
        (50..=80).contains(&total),
        "medium: expected total in 50..=80, got {total}"
    );
}

/// Large fixture: exit 0, `total` >= 200.
#[test]
fn test_harness_large_run() {
    let (output, stdout) = run_mutant("large", &["--format", "json", "--quiet", "--no-color"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "large fixture should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_harness_large_run");
    let total = v["total"]
        .as_u64()
        .expect("large: JSON missing numeric `total`") as usize;

    assert!(
        total >= 200,
        "large: expected total >= 200 mutants, got {total}"
    );
}
