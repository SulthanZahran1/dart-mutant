//! Report-format tests — Stryker JSON, JUnit XML, and HTML report generation.
//!
//! ## Contract notes
//!
//! `--format json` does two things in the binary:
//!   1. Prints the **agent** `PipelineSummary` JSON (schemaVersion "1.0") to
//!      stdout — tested in `contract.rs`, not here.
//!   2. Writes the **Stryker** mutation-testing-elements JSON
//!      (schemaVersion "2") to `<fixture>/mutation-reports/mutation-report.json`.
//!
//! `--format junit` writes `<fixture>/mutation-reports/mutation-results.xml`.
//! `--format html`  writes `<fixture>/mutation-reports/mutation-report.html`.
//!
//! These tests read the *files* written under the fixture's
//! `mutation-reports/` directory (the fixtures are real Dart projects, so the
//! tool treats the fixture path as the project root).

mod common;

use std::fs;

use common::{fixture_dir, parse_json_stdout, run_mutant};

/// Stryker JSON report file (`mutation-reports/mutation-report.json`):
/// `schemaVersion == "2"`, `files[]` present and non-empty, and each file
/// entry carries `killed`/`survived`-style status data.
#[test]
fn test_stryker_json_valid() {
    let (output, _stdout) = run_mutant("small", &["--format", "json", "--quiet", "--no-color"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "small: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stryker_path = fixture_dir("small")
        .join("mutation-reports")
        .join("mutation-report.json");
    let stryker = fs::read_to_string(&stryker_path).unwrap_or_else(|e| {
        panic!(
            "small: failed to read Stryker JSON report at {}: {e}",
            stryker_path.display()
        )
    });

    let v = parse_json_stdout(&stryker, "test_stryker_json_valid (stryker file)");

    // Stryker mutation-testing-elements schema version.
    assert_eq!(
        v["schemaVersion"], "2",
        "Stryker JSON: expected schemaVersion == \"2\""
    );

    // files[] must be present and non-empty.
    let files = v["files"]
        .as_array()
        .expect("Stryker JSON: `files` must be an array");
    assert!(
        !files.is_empty(),
        "Stryker JSON: `files` array must be non-empty"
    );

    // Each file entry has a `mutants` array; per-mutant status strings like
    // "Killed" / "Survived" must appear. We check that at least one Killed
    // and one Survived status string are present across all mutants.
    let mut saw_killed = false;
    let mut saw_survived = false;
    for f in files {
        let mutants = f["mutants"]
            .as_array()
            .expect("Stryker JSON: each file must have a `mutants` array");
        for m in mutants {
            match m["status"].as_str() {
                Some("Killed") => saw_killed = true,
                Some("Survived") => saw_survived = true,
                _ => {}
            }
        }
    }
    assert!(
        saw_killed,
        "Stryker JSON: expected at least one mutant with status \"Killed\""
    );
    assert!(
        saw_survived,
        "Stryker JSON: expected at least one mutant with status \"Survived\" \
         (equivalent mutants are mapped to Survived in the Stryker schema)"
    );
}

/// JUnit XML report (`mutation-reports/mutation-results.xml`):
/// must contain `<testsuite` or `<testsuites` and be parseable (starts with
/// `<?xml` or `<testsuite`).
#[test]
fn test_junit_xml_wellformed() {
    let (output, _) = run_mutant("small", &["--format", "junit", "--quiet", "--no-color"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "small: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let xml_path = fixture_dir("small")
        .join("mutation-reports")
        .join("mutation-results.xml");
    let xml = fs::read_to_string(&xml_path).unwrap_or_else(|e| {
        panic!(
            "small: failed to read JUnit XML report at {}: {e}",
            xml_path.display()
        )
    });

    // Must contain a testsuite(s) root.
    assert!(
        xml.contains("<testsuite") || xml.contains("<testsuites"),
        "JUnit XML: expected a `<testsuite` or `<testsuites` element"
    );

    // Light well-formedness: starts with the XML declaration or a testsuite
    // tag (allowing for a possible leading BOM we trimmed via read).
    let trimmed = xml.trim_start();
    assert!(
        trimmed.starts_with("<?xml") || trimmed.starts_with("<testsuite"),
        "JUnit XML: expected document to start with `<?xml` or `<testsuite`; \
         got prefix: {:?}",
        &trimmed[..trimmed.len().min(60)]
    );
}

/// HTML report (`mutation-reports/mutation-report.html`):
/// must be written and contain at least one of the six status names.
#[test]
fn test_html_report_generated() {
    let (output, _) = run_mutant("small", &["--format", "html", "--quiet", "--no-color"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "small: expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html_path = fixture_dir("small")
        .join("mutation-reports")
        .join("mutation-report.html");
    let html = fs::read_to_string(&html_path).unwrap_or_else(|e| {
        panic!(
            "small: failed to read HTML report at {}: {e}",
            html_path.display()
        )
    });

    // The HTML report renders status badges; at least one status name must
    // appear. Check for the human-readable forms used by `status_display`.
    let status_names = [
        "Killed",
        "Survived",
        "Timeout",
        "Equivalent",
        "Not covered",
        "Compile error",
    ];
    let found = status_names.iter().any(|name| html.contains(name));
    assert!(
        found,
        "HTML report: expected at least one of {status_names:?} in the report body"
    );
}
