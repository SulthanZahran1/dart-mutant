//! Routing tests — coverage routing must skip tests that cover no library
//! code.
//!
//! ## Contract
//!
//! The medium fixture deliberately contains a slow test file whose tests
//! sleep ~2-4s in total and cover **no** library code (the parent contract:
//! a slow test covering nothing). If the tool has no per-test coverage
//! routing and re-runs *all* tests for every mutant, each mutant pays that
//! 2-4s sleep, blowing the wall-clock budget.
//!
//! Therefore: a full medium run must complete in under 10 seconds wall-clock.
//! This is a timing-based assertion — report honestly, do not fake it.

mod common;

use std::time::Instant;

use common::{parse_and_check_schema, run_mutant};

/// A full medium run must finish in under 10 seconds wall-clock.
///
/// If the tool lacks coverage routing (re-runs all tests per mutant) this
/// fails because the slow no-coverage test adds ~2-4s per mutant across
/// 50-80 mutants.
#[test]
fn test_coverage_routing_skips_noncovering_tests() {
    let start = Instant::now();
    let (output, stdout) = run_mutant("medium", &["--format", "json", "--quiet", "--no-color"]);
    let elapsed = start.elapsed();

    // The run must still succeed.
    assert_eq!(
        output.status.code(),
        Some(0),
        "medium: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Sanity: we actually ran mutants (not an empty run that's "fast" for the
    // wrong reason).
    let v = parse_and_check_schema(&stdout, "test_coverage_routing_skips_noncovering_tests");
    let total = v["total"]
        .as_u64()
        .expect("medium: JSON missing numeric `total`") as usize;
    assert!(
        total >= 50,
        "routing test sanity: expected total >= 50 on medium, got {total}; \
         a near-empty run would make the timing assertion meaningless"
    );

    // The real assertion: wall-clock under 10s.
    assert!(
        elapsed.as_secs() < 10,
        "medium full run took {:?} — expected < 10s. \
         If the tool has no coverage routing it re-runs all tests per mutant, \
         including the slow no-coverage test (~2-4s), blowing the budget.",
        elapsed
    );
}
