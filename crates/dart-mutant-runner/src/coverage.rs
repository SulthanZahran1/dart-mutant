//! Coverage collection and routing.
//!
//! This module runs `dart test --coverage` (or `flutter test --coverage`) and
//! parses the LCOV output to build a per-test → per-line coverage map. The map
//! is used by the scheduler to route each mutant only to the tests that cover
//! the mutated line, avoiding the cost of running the full suite per mutant.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};
use log::{debug, info, warn};

use dart_mutant_core::Mutant;

// ---------------------------------------------------------------------------
// Coverage data types
// ---------------------------------------------------------------------------

/// Per-test coverage data collected from `dart test --coverage`.
///
/// Maps test names to the (file, line) pairs that test covers.
#[derive(Debug, Clone, Default)]
pub struct CoverageData {
    /// Map of test name → list of (file, line) pairs that test covers.
    pub test_coverage: HashMap<String, Vec<(String, usize)>>,
    /// Wall-clock time of the baseline test run in milliseconds.
    pub baseline_duration_ms: u64,
    /// Total number of tests discovered.
    pub total_tests: usize,
}

/// Per-mutant → list of covering tests mapping.
///
/// Built from [`CoverageData`] by finding which tests cover each mutant's
/// file and line. Used by the scheduler to route mutants only to relevant tests.
#[derive(Debug, Clone, Default)]
pub struct CoverageMap {
    /// Map of mutant ID (string) → list of test names that cover it.
    pub map: HashMap<String, Vec<String>>,
}

impl CoverageMap {
    /// Get the covering tests for a mutant ID.
    pub fn covering_tests(&self, mutant_id: &str) -> &[String] {
        self.map.get(mutant_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Number of mutants in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Coverage collection (baseline test run)
// ---------------------------------------------------------------------------

/// Run the test suite with coverage collection enabled.
///
/// Executes `dart test --coverage=coverage` (or the configured test command)
/// and parses the coverage output into a [`CoverageData`] struct. Also
/// measures the wall-clock duration of the baseline test run, which is used
/// for adaptive timeout calculation.
///
/// # Arguments
/// - `project_path` — absolute path to the Dart/Flutter project root
/// - `test_command` — the test command string (e.g. `"dart test"`, `"flutter test"`)
///
/// # Errors
/// Returns an error if the test command fails to spawn or if coverage output
/// cannot be found/parsed.
pub fn collect_coverage(project_path: &Path, test_command: &str) -> Result<CoverageData> {
    let coverage_dir = project_path.join("coverage");
    // Clean up old coverage
    if coverage_dir.exists() {
        std::fs::remove_dir_all(&coverage_dir).ok();
    }

    // Parse the test command — it may be "dart test" or "flutter test"
    let parts: Vec<&str> = test_command.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("empty test command");
    }

    let program = parts[0];
    let mut args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Coverage collection flags differ between `dart test` and `flutter test`:
    // - `dart test` accepts `--coverage=<dir>` (writes per-test JSON under
    //   `coverage/`, consumed by `parse_dart_coverage`).
    // - `flutter test` has `--coverage` as a plain bool flag plus a separate
    //   `--coverage-path=<file>` option (default `coverage/lcov.info`, consumed
    //   by `parse_lcov`). Passing `--coverage=coverage` to `flutter test`
    //   fails to parse as a boolean.
    if program == "flutter" {
        args.push("--coverage".to_string());
        args.push("--coverage-path=coverage/lcov.info".to_string());
    } else {
        args.push("--coverage=coverage".to_string());
    }

    info!(
        "Running baseline: {} {} (in {})",
        program,
        args.join(" "),
        project_path.display()
    );

    let start = Instant::now();
    let output = Command::new(program)
        .args(&args)
        .current_dir(project_path)
        .output()
        .context(format!(
            "failed to spawn `{} {}` — is the Dart/Flutter SDK on PATH?",
            program,
            args.join(" ")
        ))?;
    let elapsed = start.elapsed();

    let duration_ms = elapsed.as_millis() as u64;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        warn!(
            "baseline test run failed (exit {:?})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            output.status.code()
        );
        // Even if tests fail, we can still parse coverage
    }

    // Parse LCOV output
    let lcov_path = coverage_dir.join("lcov.info");
    let test_coverage = if lcov_path.exists() {
        let lcov_content = std::fs::read_to_string(&lcov_path)
            .context(format!("failed to read {}", lcov_path.display()))?;
        parse_lcov(&lcov_content)
    } else {
        // Dart's `dart test --coverage` may produce coverage in a different format.
        // Look for coverage/*.json files (Dart VM coverage format)
        parse_dart_coverage(&coverage_dir).unwrap_or_default()
    };

    // Try to extract test names from stdout
    let test_names = extract_test_names(&String::from_utf8_lossy(&output.stdout));
    let total_tests = test_names.len();

    // If we got coverage data but no test names, create synthetic test names
    if test_coverage.is_empty() && total_tests == 0 {
        warn!(
            "no coverage data found in {} — coverage routing will be disabled",
            coverage_dir.display()
        );
    }

    Ok(CoverageData {
        test_coverage,
        baseline_duration_ms: duration_ms,
        total_tests,
    })
}

// ---------------------------------------------------------------------------
// LCOV parser
// ---------------------------------------------------------------------------

/// Parse LCOV format coverage data into a per-test → per-line map.
///
/// LCOV format:
/// ```text
/// SF:lib/src/math_utils.dart
/// DA:1,1
/// DA:2,1
/// ...
/// end_of_record
/// ```
///
/// Since LCOV doesn't directly associate coverage with individual tests (it
/// aggregates across the whole test run), we map all covered lines to a
/// single "baseline" test. This means coverage routing falls back to running
/// all tests for each mutant — but we still benefit from the adaptive timeout
/// and caching.
///
/// For per-test coverage, Dart's `--coverage` output in JSON format would need
/// to be parsed separately. The LCOV format gives us file → line coverage.
fn parse_lcov(lcov_content: &str) -> HashMap<String, Vec<(String, usize)>> {
    let mut coverage: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    let mut current_file = String::new();
    let mut covered_lines: Vec<(String, usize)> = Vec::new();

    for line in lcov_content.lines() {
        let line = line.trim();
        if let Some(sf) = line.strip_prefix("SF:") {
            // Source file
            current_file = sf.to_string();
            covered_lines.clear();
        } else if let Some(rest) = line.strip_prefix("DA:") {
            // DA:line_number,execution_count[,checksum]
            let parts: Vec<&str> = rest.split(',').collect();
            if let Some(line_str) = parts.first() {
                if let Ok(line_num) = line_str.parse::<usize>() {
                    // Only include lines that were actually executed (count > 0)
                    let count: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    if count > 0 {
                        covered_lines.push((current_file.clone(), line_num));
                    }
                }
            }
        } else if line == "end_of_record" {
            if !current_file.is_empty() && !covered_lines.is_empty() {
                // LCOV aggregates across all tests — we map to a "baseline" test.
                // In a real per-test coverage setup, we'd need Dart's JSON format.
                coverage
                    .entry("baseline".to_string())
                    .or_default()
                    .append(&mut covered_lines);
            }
            current_file.clear();
        }
    }

    // Normalize file paths — LCOV may use absolute paths; convert to relative
    let normalized: HashMap<String, Vec<(String, usize)>> = coverage
        .into_iter()
        .map(|(test, lines)| {
            let normalized_lines: Vec<(String, usize)> = lines
                .into_iter()
                .map(|(file, line)| (normalize_path(&file), line))
                .collect();
            (test, normalized_lines)
        })
        .collect();

    normalized
}

/// Normalize a file path to be relative (strip leading `./`, absolute prefixes).
fn normalize_path(path: &str) -> String {
    // Windows coverage JSON emits absolute paths with backslash separators
    // (e.g. `C:\repo\lib\foo.dart`). Normalize them to forward slashes so
    // suffix matching against tree-sitter's `/`-separated `file_path` works
    // on all platforms.
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");
    // Strip `package:` URIs from Dart coverage: `package:math_utils/math_utils.dart`
    if let Some(pkg) = path.strip_prefix("package:") {
        // Keep only the path after the package name
        if let Some(idx) = pkg.find('/') {
            return pkg[idx + 1..].to_string();
        }
        return pkg.to_string();
    }
    // If it's an absolute path, try to make it relative by taking the last
    // few components (lib/...)
    if path.starts_with('/') {
        if let Some(idx) = path.find("/lib/") {
            return path[idx + 1..].to_string();
        }
        if let Some(idx) = path.find("/test/") {
            return path[idx + 1..].to_string();
        }
    }
    path.to_string()
}

/// Parse Dart VM coverage JSON files (from `dart test --coverage=coverage`).
///
/// Dart 3.12+ emits coverage as JSON files (one per test file) in
/// `coverage/test/*.json`. The format is:
/// ```json
/// {"type":"CodeCoverage","coverage":[
///   {"source":"package:math_utils/math_utils.dart","hits":[5,2,8,2,11,2,...]}
/// ]}
/// ```
/// where `hits` is a FLAT array of alternating (line, hit_count) pairs.
/// Only lines with hit_count > 0 are covered.
fn parse_dart_coverage(coverage_dir: &Path) -> Option<HashMap<String, Vec<(String, usize)>>> {
    let mut coverage: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    // JSON files may be nested in subdirectories (e.g. coverage/test/)
    let mut json_files: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![coverage_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    json_files.push(path);
                }
            }
        }
    }

    for path in json_files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };

        // Each JSON file = one test file's coverage → key by the test path
        // relative to the project root (strip the `coverage/` prefix), e.g.
        // `coverage/test/math_utils_test.dart.vm.json` → `test/math_utils_test.dart`.
        let rel = path
            .strip_prefix(coverage_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let test_name = rel
            .trim_start_matches('/')
            .trim_end_matches(".vm.json")
            .to_string();

        if let Some(coverage_arr) = json.get("coverage").and_then(|c| c.as_array()) {
            for entry in coverage_arr {
                let file = entry
                    .get("source")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(hits) = entry.get("hits").and_then(|h| h.as_array()) {
                    // hits is a FLAT array: [line1, count1, line2, count2, ...]
                    let mut iter = hits.iter();
                    while let Some(line_v) = iter.next() {
                        let Some(count_v) = iter.next() else { break };
                        let line = line_v.as_u64().unwrap_or(0) as usize;
                        let count = count_v.as_u64().unwrap_or(0);
                        if line > 0 && count > 0 {
                            coverage
                                .entry(test_name.clone())
                                .or_default()
                                .push((normalize_path(&file), line));
                        }
                    }
                }
            }
        }
    }

    if coverage.is_empty() {
        None
    } else {
        Some(coverage)
    }
}

/// Extract test names from `dart test` stdout output.
///
/// `dart test` output format:
/// ```text
/// ✓ test name 1
/// ✓ test name 2
/// Exited with status code 0
/// ```
fn extract_test_names(stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        // Dart test reporter: "✓ test name" or "test name: [OK/FAIL]"
        if let Some(name) = trimmed.strip_prefix("✓ ") {
            names.push(name.trim().to_string());
        } else if let Some(name) = trimmed.strip_prefix("✗ ") {
            names.push(name.trim().to_string());
        }
    }
    names
}

// ---------------------------------------------------------------------------
// Coverage map builder
// ---------------------------------------------------------------------------

/// Build a per-mutant → covering-tests map from coverage data.
///
/// For each mutant, find all tests whose coverage touches the mutant's
/// file and line. Mutants with no covering tests will get an empty list,
/// which the scheduler will interpret as `NotCovered`.
///
/// # Arguments
/// - `coverage` — the coverage data from the baseline test run
/// - `mutants` — the list of mutants to build the map for
pub fn build_coverage_map(coverage: &CoverageData, mutants: &[Mutant]) -> Result<CoverageMap> {
    let mut map = HashMap::new();

    for m in mutants {
        let covering: Vec<String> = coverage
            .test_coverage
            .iter()
            .filter_map(|(test_name, lines)| {
                // Check if any (file, line) pair in this test's coverage
                // matches the mutant's file and line
                let covers = lines.iter().any(|(file, line)| {
                    // A test covers a mutant only if BOTH the file matches
                    // (exact or normalized) AND the line is in the executed set.
                    let path_matches = *file == m.file_path
                        || normalize_path(file) == m.file_path
                        || file.ends_with(&m.file_path)
                        || m.file_path.ends_with(file);
                    path_matches && *line == m.line
                });
                if covers {
                    Some(test_name.clone())
                } else {
                    None
                }
            })
            .collect();

        debug!(
            "Mutant {} at {}:{} → {} covering tests",
            m.id,
            m.file_path,
            m.line,
            covering.len()
        );
        map.insert(m.id.clone(), covering);
    }

    Ok(CoverageMap { map })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_coverage_map() {
        let mutants: Vec<Mutant> = vec![];
        let coverage = CoverageData::default();
        let map = build_coverage_map(&coverage, &mutants).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_lcov_basic() {
        let lcov = "\
SF:lib/src/math.dart
DA:1,1
DA:2,0
DA:3,1
end_of_record
SF:lib/src/calc.dart
DA:5,2
end_of_record
";
        let coverage = parse_lcov(lcov);
        // LCOV maps all to "baseline" test
        let baseline = coverage.get("baseline").unwrap();
        // DA:2 has count 0 → excluded
        assert!(baseline.contains(&("lib/src/math.dart".to_string(), 1)));
        assert!(!baseline.contains(&("lib/src/math.dart".to_string(), 2)));
        assert!(baseline.contains(&("lib/src/math.dart".to_string(), 3)));
        assert!(baseline.contains(&("lib/src/calc.dart".to_string(), 5)));
    }

    #[test]
    fn test_parse_lcov_empty() {
        let coverage = parse_lcov("");
        assert!(coverage.is_empty());
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("./lib/x.dart"), "lib/x.dart");
        assert_eq!(normalize_path("lib/x.dart"), "lib/x.dart");
        assert_eq!(normalize_path("/home/user/lib/x.dart"), "lib/x.dart");
        assert_eq!(normalize_path("/home/user/test/x.dart"), "test/x.dart");
    }

    #[test]
    fn test_extract_test_names() {
        let stdout = "✓ first test\n✗ second test\nSome other line\n";
        let names = extract_test_names(stdout);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "first test");
        assert_eq!(names[1], "second test");
    }

    #[test]
    fn test_coverage_map_for_mutant() {
        let coverage = CoverageData {
            test_coverage: {
                let mut m = HashMap::new();
                m.insert(
                    "test1".to_string(),
                    vec![
                        ("lib/math.dart".to_string(), 10),
                        ("lib/math.dart".to_string(), 11),
                    ],
                );
                m.insert("test2".to_string(), vec![("lib/calc.dart".to_string(), 5)]);
                m
            },
            baseline_duration_ms: 1000,
            total_tests: 2,
        };

        let mutants = vec![
            Mutant::new("0", "lib/math.dart", 10, 1, "AOR", "+", "-", "test"),
            Mutant::new("1", "lib/math.dart", 20, 1, "ROR", ">", "<", "test"),
            Mutant::new("2", "lib/calc.dart", 5, 1, "SDL", "x", "", "test"),
        ];

        let map = build_coverage_map(&coverage, &mutants).unwrap();
        assert_eq!(map.covering_tests("0"), &["test1"]);
        assert_eq!(map.covering_tests("1"), &[] as &[String]); // no coverage
        assert_eq!(map.covering_tests("2"), &["test2"]);
    }
}
