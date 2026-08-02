//! Correctness tests — mutation classification correctness, compile-error
//! rate, and the threshold / error exit-code contract.
//!
//! These verify the *semantics* of the tool's output, not just that it runs.

mod common;

use common::{parse_and_check_schema, run_mutant, run_mutant_raw};

/// On both the medium and large fixtures, the compile-error rate
/// (`compileError / total`) must be strictly under 2%.
///
/// A higher rate means the tree-sitter grammar or the mutation operators are
/// producing invalid Dart — a bug, not a test-quality issue.
#[test]
fn test_compile_error_rate_under_2pct() {
    for fixture in &["medium", "large"] {
        let (output, stdout) = run_mutant(fixture, &["--format", "json", "--quiet", "--no-color"]);

        assert_eq!(
            output.status.code(),
            Some(0),
            "{fixture}: expected exit 0; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let v = parse_and_check_schema(&stdout, "test_compile_error_rate_under_2pct");
        let total = v["total"]
            .as_u64()
            .expect("{fixture}: JSON missing numeric `total`") as usize;
        let compile_error = v["compileError"]
            .as_u64()
            .expect("{fixture}: JSON missing numeric `compileError` field")
            as usize;

        // The field must always exist (contract); guard against divide-by-zero.
        assert!(total > 0, "{fixture}: total must be > 0 for rate check");
        let rate = compile_error as f64 / total as f64;
        assert!(
            rate < 0.02,
            "{fixture}: compileError/total = {compile_error}/{total} = {rate:.4} \
             must be < 0.02"
        );
    }
}

/// The medium fixture must produce at least one of each of KILLED, SURVIVED,
/// and TIMEOUT. The fields for equivalent / notCovered / compileError must
/// *exist* in the JSON (they may be zero — the contract only requires the
/// field presence, not a non-zero count).
#[test]
fn test_all_six_statuses() {
    let (output, stdout) = run_mutant("medium", &["--format", "json", "--quiet", "--no-color"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "medium: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v = parse_and_check_schema(&stdout, "test_all_six_statuses");

    // The three required non-zero statuses.
    let killed = v["killed"]
        .as_u64()
        .expect("medium: JSON missing numeric `killed`");
    let survived = v["survived"]
        .as_u64()
        .expect("medium: JSON missing numeric `survived`");
    let timeout = v["timeout"]
        .as_u64()
        .expect("medium: JSON missing numeric `timeout`");
    let not_covered = v["notCovered"].as_u64().unwrap_or(0);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        killed >= 1,
        "medium: expected at least 1 KILLED, got {killed} \
         (survived={survived} timeout={timeout} notCovered={not_covered}); \
         full JSON: {stdout}\nstderr: {stderr}"
    );
    assert!(
        survived >= 1,
        "medium: expected at least 1 SURVIVED, got {survived} \
         (killed={killed} timeout={timeout} notCovered={not_covered}); \
         full JSON: {stdout}"
    );
    assert!(
        timeout >= 1,
        "medium: expected at least 1 TIMEOUT, got {timeout} \
         (killed={killed} survived={survived} notCovered={not_covered}); \
         full JSON: {stdout}"
    );

    // The three fields that may be zero but MUST exist (be present + numeric).
    for field in &["equivalent", "notCovered", "compileError"] {
        assert!(
            v[*field].is_number(),
            "medium: JSON field `{field}` must exist and be numeric \
             (may be 0, but the field is required by the contract)"
        );
    }
}

/// Threshold / error exit-code contract:
/// - medium with `--threshold 99` → exit 1 (MSI below threshold; medium is
///   MSI 88.9, not 100 — the small fixture is deliberately 100% killed so it
///   would always pass a 99% threshold)
/// - small with `--threshold 0`  → exit 0 (always passes)
/// - a nonexistent `--path`      → exit 2 (error)
#[test]
fn test_threshold_exit_codes() {
    // 99% threshold: medium's MSI (88.9) is below 99% → exit 1.
    let (out_above, _) = run_mutant("medium", &["--threshold", "99", "--quiet"]);
    assert_eq!(
        out_above.status.code(),
        Some(1),
        "medium --threshold 99: expected exit 1 (below threshold)"
    );

    // 0% threshold: anything passes → exit 0.
    let (out_zero, _) = run_mutant("small", &["--threshold", "0", "--quiet"]);
    assert_eq!(
        out_zero.status.code(),
        Some(0),
        "small --threshold 0: expected exit 0"
    );

    // Nonexistent path → exit 2 (error).
    let (out_err, _) = run_mutant_raw("/nonexistent/path/that/does/not/exist", &["--quiet"]);
    assert_eq!(
        out_err.status.code(),
        Some(2),
        "nonexistent --path: expected exit 2 (error); stderr: {}",
        String::from_utf8_lossy(&out_err.stderr)
    );
}
