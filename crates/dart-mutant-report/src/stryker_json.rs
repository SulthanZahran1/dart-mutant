//! Stryker mutation-testing-elements compatible JSON report generator.
//!
//! Produces JSON that validates against the [mutation-testing-elements](https://github.com/stryker-mutator/mutation-testing-elements) schema.
//!
//! # Schema
//!
//! ```json
//! {
//!   "schemaVersion": "2",
//!   "thresholds": { "high": 80, "low": 20 },
//!   "files": [
//!     {
//!       "file": "lib/src/math_utils.dart",
//!       "mutants": [
//!         {
//!           "id": "0",
//!           "location": { "start": { "line": 1, "column": 1 }, "end": { "line": 1, "column": 5 } },
//!           "mutatorName": "AOR",
//!           "status": "Killed",
//!           "killedByTests": ["test_add"]
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```

use anyhow::Result;
use serde::Serialize;

use crate::{stryker_status, MutantResult};

// ---------------------------------------------------------------------------
// JSON schema types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrykerReport {
    schema_version: String,
    thresholds: Thresholds,
    files: Vec<StrykerFile>,
}

#[derive(Serialize)]
struct Thresholds {
    high: u32,
    low: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrykerFile {
    file: String,
    mutants: Vec<StrykerMutant>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrykerMutant {
    id: String,
    location: StrykerLocation,
    mutator_name: String,
    status: String,
    killed_by_tests: Vec<String>,
}

#[derive(Serialize)]
struct StrykerLocation {
    start: StrykerPosition,
    end: StrykerPosition,
}

#[derive(Serialize)]
struct StrykerPosition {
    line: u32,
    column: u32,
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Generate a Stryker-compatible JSON report from mutation results.
///
/// Mutants are grouped by source file. Equivalent mutants are reported as
/// `Survived` (the Stryker schema has no `Equivalent` status — see
/// [`stryker_status`]).
///
/// # Example
///
/// ```
/// use dart_mutant_core::{Mutant, MutantResult, MutantStatus};
/// use dart_mutant_report::stryker_json;
///
/// let mutant = Mutant {
///     id: "0".to_string(),
///     file_path: "lib/math.dart".to_string(),
///     line: 1,
///     column: 1,
///     operator: "AOR".to_string(),
///     original: "a + b".to_string(),
///     replacement: "a - b".to_string(),
///     description: "AOR: + to -".to_string(),
/// };
/// let results = vec![MutantResult {
///     mutant, status: MutantStatus::Killed,
///     covering_tests: vec![], message: None,
/// }];
///
/// let json = stryker_json::generate(&results).unwrap();
/// assert!(json.contains("\"schemaVersion\": \"2\""));
/// ```
pub fn generate(results: &[MutantResult]) -> Result<String> {
    // Group mutants by file, preserving insertion order (first-seen).
    let mut files: Vec<String> = Vec::new();
    let mut file_mutants: Vec<Vec<&MutantResult>> = Vec::new();

    for result in results {
        if let Some(pos) = files.iter().position(|f| f == &result.mutant.file_path) {
            file_mutants[pos].push(result);
        } else {
            files.push(result.mutant.file_path.clone());
            file_mutants.push(vec![result]);
        }
    }

    let stryker_files: Vec<StrykerFile> = files
        .iter()
        .zip(file_mutants.iter())
        .map(|(file, mutants)| StrykerFile {
            file: file.clone(),
            mutants: mutants
                .iter()
                .map(|r| StrykerMutant {
                    id: r.mutant.id.clone(),
                    location: StrykerLocation {
                        start: StrykerPosition {
                            line: r.mutant.line as u32,
                            column: r.mutant.column as u32,
                        },
                        end: StrykerPosition {
                            line: r.mutant.line as u32,
                            column: r.mutant.column as u32,
                        },
                    },
                    mutator_name: r.mutant.operator.clone(),
                    status: stryker_status(r.status).to_string(),
                    killed_by_tests: r.covering_tests.clone(),
                })
                .collect(),
        })
        .collect();

    let report = StrykerReport {
        schema_version: "2".to_string(),
        thresholds: Thresholds { high: 80, low: 20 },
        files: stryker_files,
    };

    Ok(serde_json::to_string_pretty(&report)?)
}

/// Generate a Stryker-compatible JSON report and write it to a file.
pub fn generate_to_file(results: &[MutantResult], path: &std::path::Path) -> Result<()> {
    let json = generate(results)?;
    crate::write_report_to_file(path, &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dart_mutant_core::{Mutant, MutantStatus};

    fn make_result(id: &str, file: &str, status: MutantStatus) -> MutantResult {
        MutantResult {
            mutant: Mutant {
                id: id.to_string(),
                file_path: file.to_string(),
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
    fn test_generates_valid_json() {
        let results = vec![make_result("0", "lib/math.dart", MutantStatus::Killed)];
        let json = generate(&results).unwrap();
        assert!(json.contains("\"schemaVersion\": \"2\""));
        assert!(json.contains("\"thresholds\""));
        assert!(json.contains("\"high\": 80"));
        assert!(json.contains("\"low\": 20"));
    }

    #[test]
    fn test_groups_by_file() {
        let results = vec![
            make_result("0", "lib/a.dart", MutantStatus::Killed),
            make_result("1", "lib/b.dart", MutantStatus::Survived),
            make_result("2", "lib/a.dart", MutantStatus::Killed),
        ];
        let json = generate(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let files = parsed["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_killed_by_tests_included() {
        let mut result = make_result("0", "lib/math.dart", MutantStatus::Killed);
        result.covering_tests = vec!["test_add".to_string(), "test_sub".to_string()];
        let json = generate(&[result]).unwrap();
        assert!(json.contains("test_add"));
        assert!(json.contains("test_sub"));
    }

    #[test]
    fn test_status_mapping() {
        let results = vec![
            make_result("0", "lib/a.dart", MutantStatus::Killed),
            make_result("1", "lib/a.dart", MutantStatus::Survived),
            make_result("2", "lib/a.dart", MutantStatus::Timeout),
            make_result("3", "lib/a.dart", MutantStatus::NotCovered),
            make_result("4", "lib/a.dart", MutantStatus::CompileError),
        ];
        let json = generate(&results).unwrap();
        assert!(json.contains("\"Killed\""));
        assert!(json.contains("\"Survived\""));
        assert!(json.contains("\"Timeout\""));
        assert!(json.contains("\"NoCoverage\""));
        assert!(json.contains("\"CompileError\""));
    }

    #[test]
    fn test_empty_results() {
        let json = generate(&[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["files"].as_array().unwrap().len(), 0);
    }
}
