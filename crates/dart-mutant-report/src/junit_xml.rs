//! JUnit XML report generator.
//!
//! Produces standard JUnit XML for CI test result panels (GitHub Actions,
//! GitLab CI, Azure Pipelines, Jenkins). Each mutant appears as a test case
//! with pass/fail status.
//!
//! # Structure
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <testsuites>
//!   <testsuite name="dart_mutant" tests="3" failures="1" errors="1" skipped="1" time="0">
//!     <testcase name="mutant_0" classname="lib/math.dart.AOR" time="0" />
//!     <testcase name="mutant_1" classname="lib/math.dart.AOR" time="0">
//!       <failure message="Mutant survived">...</failure>
//!     </testcase>
//!     <testcase name="mutant_2" classname="lib/math.dart.AOR" time="0">
//!       <error message="Mutant timed out">...</error>
//!     </testcase>
//!     <testcase name="mutant_3" classname="lib/math.dart.AOR" time="0">
//!       <skipped message="No test coverage" />
//!     </testcase>
//!   </testsuite>
//! </testsuites>
//! ```

use anyhow::Result;

use crate::MutantResult;
use crate::MutantStatus;

// ---------------------------------------------------------------------------
// XML escaping
// ---------------------------------------------------------------------------

/// Escape special XML characters in a string.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Generate a JUnit XML report from mutation results.
///
/// - **Killed** → passing testcase (no child element)
/// - **Survived** → `<failure>` child element
/// - **Timeout** → `<error>` child element
/// - **Equivalent** → `<skipped>` (mutation is unkillable)
/// - **NotCovered** → `<skipped>` (no test coverage)
/// - **CompileError** → `<error>` child element
///
/// # Example
///
/// ```
/// use dart_mutant_core::{Mutant, MutantResult, MutantStatus};
/// use dart_mutant_report::junit_xml;
///
/// let results = vec![MutantResult {
///     mutant: Mutant {
///         id: "0".to_string(), file_path: "lib/math.dart".to_string(),
///         line: 1, column: 1,
///         operator: "AOR".to_string(),
///         original: "a + b".to_string(), replacement: "a - b".to_string(),
///         description: "AOR: + to -".to_string(),
///     },
///     status: MutantStatus::Killed,
///     covering_tests: vec![], message: None,
/// }];
///
/// let xml = junit_xml::generate(&results).unwrap();
/// assert!(xml.contains("<testsuites>"));
/// ```
pub fn generate(results: &[MutantResult]) -> Result<String> {
    let total = results.len();
    let failures = results
        .iter()
        .filter(|r| matches!(r.status, MutantStatus::Survived))
        .count();
    let errors = results
        .iter()
        .filter(|r| matches!(r.status, MutantStatus::Timeout | MutantStatus::CompileError))
        .count();
    let skipped = results
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                MutantStatus::Equivalent | MutantStatus::NotCovered
            )
        })
        .count();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuites>\n  <testsuite name=\"dart_mutant\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"0\">\n",
        total, failures, errors, skipped
    ));

    for r in results {
        let classname = format!(
            "{}.{}",
            xml_escape(&r.mutant.file_path),
            xml_escape(&r.mutant.operator)
        );
        let name = format!("mutant_{}", xml_escape(&r.mutant.id));

        xml.push_str(&format!(
            "    <testcase name=\"{}\" classname=\"{}\" time=\"0\"",
            name, classname
        ));

        match r.status {
            MutantStatus::Killed => {
                // Passing test — self-closing tag
                xml.push_str(" />\n");
            }
            MutantStatus::Survived => {
                xml.push_str(">\n");
                let msg = format!(
                    "Mutant {} survived: {} → {} (line {})",
                    r.mutant.id, r.mutant.original, r.mutant.replacement, r.mutant.line
                );
                let escaped_msg = xml_escape(&msg);
                xml.push_str(&format!(
                    "      <failure message=\"{}\">{}</failure>\n",
                    escaped_msg, escaped_msg
                ));
                xml.push_str("    </testcase>\n");
            }
            MutantStatus::Timeout => {
                xml.push_str(">\n");
                let msg = format!(
                    "Mutant {} timed out: {} → {} (line {})",
                    r.mutant.id, r.mutant.original, r.mutant.replacement, r.mutant.line
                );
                let escaped_msg = xml_escape(&msg);
                xml.push_str(&format!(
                    "      <error message=\"{}\">{}</error>\n",
                    escaped_msg, escaped_msg
                ));
                xml.push_str("    </testcase>\n");
            }
            MutantStatus::Equivalent => {
                xml.push_str(">\n");
                xml.push_str("      <skipped message=\"Equivalent mutant — unkillable\" />\n");
                xml.push_str("    </testcase>\n");
            }
            MutantStatus::NotCovered => {
                xml.push_str(">\n");
                xml.push_str("      <skipped message=\"No test coverage for this line\" />\n");
                xml.push_str("    </testcase>\n");
            }
            MutantStatus::CompileError => {
                xml.push_str(">\n");
                let msg = format!(
                    "Mutant {} failed to compile: {} → {} (line {})",
                    r.mutant.id, r.mutant.original, r.mutant.replacement, r.mutant.line
                );
                let escaped_msg = xml_escape(&msg);
                xml.push_str(&format!(
                    "      <error message=\"{}\">{}</error>\n",
                    escaped_msg, escaped_msg
                ));
                xml.push_str("    </testcase>\n");
            }
        }
    }

    xml.push_str("  </testsuite>\n</testsuites>\n");

    Ok(xml)
}

/// Generate a JUnit XML report and write it to a file.
pub fn generate_to_file(results: &[MutantResult], path: &std::path::Path) -> Result<()> {
    let xml = generate(results)?;
    crate::write_report_to_file(path, &xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dart_mutant_core::{Mutant, MutantStatus};

    fn make_result(id: &str, status: MutantStatus) -> MutantResult {
        MutantResult {
            mutant: Mutant {
                id: id.to_string(),
                file_path: "lib/math.dart".to_string(),
                line: 1,
                column: 1,
                operator: "AOR".to_string(),
                original: "a + b".to_string(),
                replacement: "a - b".to_string(),
                description: "AOR: + → -".to_string(),
            },
            status,
            covering_tests: vec![],
            message: None,
        }
    }

    #[test]
    fn test_xml_structure() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<testsuites>"));
        assert!(xml.contains("</testsuites>"));
        assert!(xml.contains("<testsuite"));
        assert!(xml.contains("</testsuite>"));
    }

    #[test]
    fn test_killed_is_passing() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<testcase"));
        assert!(!xml.contains("<failure"));
        assert!(!xml.contains("<error"));
        assert!(!xml.contains("<skipped"));
    }

    #[test]
    fn test_survived_is_failure() {
        let results = vec![make_result("0", MutantStatus::Survived)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<failure"));
    }

    #[test]
    fn test_timeout_is_error() {
        let results = vec![make_result("0", MutantStatus::Timeout)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<error"));
    }

    #[test]
    fn test_not_covered_is_skipped() {
        let results = vec![make_result("0", MutantStatus::NotCovered)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<skipped"));
    }

    #[test]
    fn test_equivalent_is_skipped() {
        let results = vec![make_result("0", MutantStatus::Equivalent)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<skipped"));
    }

    #[test]
    fn test_compile_error_is_error() {
        let results = vec![make_result("0", MutantStatus::CompileError)];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("<error"));
    }

    #[test]
    fn test_counts_in_testsuite() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::Timeout),
            make_result("3", MutantStatus::NotCovered),
        ];
        let xml = generate(&results).unwrap();
        assert!(xml.contains("tests=\"4\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("errors=\"1\""));
        assert!(xml.contains("skipped=\"1\""));
    }

    #[test]
    fn test_xml_escaping() {
        let mut r = make_result("0", MutantStatus::Survived);
        r.mutant.original = "a < b && c > d".to_string();
        r.mutant.replacement = "a > b || c < d".to_string();
        let xml = generate(&[r]).unwrap();
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
        assert!(xml.contains("&amp;"));
    }

    #[test]
    fn test_empty_results() {
        let xml = generate(&[]).unwrap();
        assert!(xml.contains("tests=\"0\""));
        assert!(xml.contains("<testsuites>"));
    }
}
