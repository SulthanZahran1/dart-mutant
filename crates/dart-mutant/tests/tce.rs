//! TCE (Trivial Compiler Equivalence) tests — `--detect-equivalent` runs the
//! equivalent-mutant detection pass and the MSI is recomputed with
//! equivalents excluded.
//!
//! ## Contract
//!
//! `--detect-equivalent` on the medium fixture:
//!   - exit 0,
//!   - JSON parses,
//!   - `equivalent` count >= 0 (may be zero — the field just must exist and
//!     not crash),
//!   - the tool does not crash,
//!   - MSI is computed (the `mutationScore` field is present and numeric).

mod common;

use common::{parse_and_check_schema, run_mutant};

/// `--detect-equivalent` on medium: no crash, JSON parses, `equivalent` is a
/// non-negative number, MSI is computed.
#[test]
fn test_equivalent_detection_runs() {
    let (output, stdout) = run_mutant(
        "medium",
        &[
            "--detect-equivalent",
            "--format",
            "json",
            "--quiet",
            "--no-color",
        ],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "medium --detect-equivalent: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_equivalent_detection_runs");

    // `equivalent` must exist and be a non-negative integer.
    let equivalent = v["equivalent"]
        .as_u64()
        .expect("medium --detect-equivalent: JSON missing numeric `equivalent` field");
    assert!(
        equivalent < u64::MAX,
        "medium --detect-equivalent: `equivalent` is not a sane number ({equivalent})"
    );
    // equivalent >= 0 is trivially true for u64; the real contract is that
    // the field exists and the run didn't crash — both already asserted.

    // MSI must be computed and present (numeric).
    let msi = v["mutationScore"]
        .as_f64()
        .expect("medium --detect-equivalent: JSON missing numeric `mutationScore`");
    assert!(
        (0.0..=100.0).contains(&msi),
        "medium --detect-equivalent: mutationScore {msi} must be in [0.0, 100.0]"
    );
}
