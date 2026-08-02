//! Scale tests — cold vs warm run timing on the large fixture.
//!
//! These are `#[ignore]` by default and run explicitly in CI (e.g.
//! `cargo test -- --ignored scale`). They assert the performance budget from
//! the AGENTS.md "Performance is a feature" table:
//!
//! | Metric | Target |
//! | Cold run (500 mutants, 50 files) | < 10 minutes |
//! | Warm rerun (same source + tests) | < 30 seconds |
//!
//! The warm-run speedup comes from the content-addressed incremental cache
//! in `dart-mutant-runner::cache`.

mod common;

use std::time::Instant;

use common::{parse_and_check_schema, run_mutant};

/// Cold run on the large fixture: must complete in under 600 seconds
/// (10 minutes).
#[test]
#[ignore = "scale test — run explicitly in CI: cargo test -- --ignored scale"]
fn test_large_cold_under_10min() {
    let start = Instant::now();
    let (output, stdout) = run_mutant("large", &["--format", "json", "--quiet", "--no-color"]);
    let elapsed = start.elapsed();

    assert_eq!(
        output.status.code(),
        Some(0),
        "large cold: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_large_cold_under_10min");
    let total = v["total"]
        .as_u64()
        .expect("large: JSON missing numeric `total`") as usize;
    assert!(
        total >= 200,
        "large cold: expected total >= 200, got {total}"
    );

    assert!(
        elapsed.as_secs() < 600,
        "large cold run took {:?} — expected < 600s (10 min)",
        elapsed
    );
}

/// Warm rerun: a second immediate run on the large fixture must complete in
/// under 30 seconds. The content-addressed cache makes the second run warm.
#[test]
#[ignore = "scale test — run explicitly in CI: cargo test -- --ignored scale"]
fn test_large_warm_under_30s() {
    // Prime the cache with a first run (not timed).
    let (first_out, _) = run_mutant("large", &["--format", "json", "--quiet", "--no-color"]);
    assert_eq!(
        first_out.status.code(),
        Some(0),
        "large warm (prime): expected exit 0; stderr: {}",
        String::from_utf8_lossy(&first_out.stderr)
    );

    // Timed warm run.
    let start = Instant::now();
    let (output, stdout) = run_mutant("large", &["--format", "json", "--quiet", "--no-color"]);
    let elapsed = start.elapsed();

    assert_eq!(
        output.status.code(),
        Some(0),
        "large warm: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _v = parse_and_check_schema(&stdout, "test_large_warm_under_30s");

    assert!(
        elapsed.as_secs() < 30,
        "large warm run took {:?} — expected < 30s (content-addressed cache warm)",
        elapsed
    );
}
