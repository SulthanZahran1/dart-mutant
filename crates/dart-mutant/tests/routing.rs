//! Routing tests — coverage routing must skip tests that cover no library
//! code.
//!
//! ## Contract
//!
//! The medium fixture contains `test/routing_probe_test.dart`: a test that
//! imports NO library code (covers zero lib lines) and, when the
//! `DM_ROUTING_MARKER` env var is set, appends a marker line to that file
//! path every time it executes.
//!
//! Under per-test coverage routing the probe runs exactly **once** (during
//! baseline coverage collection) and is never re-run for any mutant, because
//! no mutant is covered by it. If the tool lacks routing and re-runs the full
//! suite per mutant, the probe runs once per mutant, appending one marker
//! line each time.
//!
//! So the assertion is **behavioral and machine-independent**: after a full
//! medium run, the marker file must contain exactly one line. A timing
//! assertion would be flaky (per-mutant JIT spawns dominate wall-clock on
//! loaded machines), so we count executions instead.

mod common;

use std::io::Read;
use std::path::PathBuf;

use common::{parse_and_check_schema, run_mutant_with_env};

/// Marker file path in the system temp dir, unique per test process.
fn marker_path() -> PathBuf {
    std::env::temp_dir().join(format!("dm_routing_marker_{}.txt", std::process::id()))
}

fn count_marker_lines() -> usize {
    let p = marker_path();
    if !p.exists() {
        return 0;
    }
    let mut s = String::new();
    std::fs::File::open(&p)
        .expect("marker file exists")
        .read_to_string(&mut s)
        .expect("read marker");
    s.lines().count()
}

/// A full medium run with the routing probe in place must execute the probe
/// exactly once (baseline only), proving non-covering tests are not re-run
/// per mutant.
#[test]
fn test_coverage_routing_skips_noncovering_tests() {
    // Clean slate: remove any marker from previous runs.
    let marker = marker_path();
    let _ = std::fs::remove_file(&marker);

    // Run the full medium fixture with the marker env var set. The env var
    // is inherited by the tool's child `dart test` processes, so the probe
    // test appends a line every time it executes.
    let (output, stdout) = run_mutant_with_env(
        "medium",
        &["--format", "json", "--quiet", "--no-color"],
        &[(
            "DM_ROUTING_MARKER",
            marker.to_str().expect("marker path utf8"),
        )],
    );

    // The run must succeed.
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
         a near-empty run would make the routing assertion meaningless"
    );

    // The real assertion: the probe ran exactly once (baseline coverage
    // collection). If the tool re-ran the full suite per mutant, it would
    // have run total+1 times.
    let runs = count_marker_lines();
    assert_eq!(
        runs, 1,
        "routing probe executed {runs} times; expected exactly 1 (baseline only). \
         If it ran {}+ times the tool is re-running non-covering tests per mutant \
         (no per-test coverage routing).",
        total
    );

    // Clean up the marker.
    let _ = std::fs::remove_file(&marker);
}
