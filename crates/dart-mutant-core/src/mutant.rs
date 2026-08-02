//! Mutant types: Mutant, MutantId, MutationPoint, MutantStatus.
//!
//! These types represent the mutation testing primitives used throughout
//! dart-mutant-core. A [`Mutant`] describes a single mutation that can be
//! applied to a source file. [`MutantStatus`] tracks the outcome of running
//! the test suite against a mutant.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The outcome of running the test suite against a single mutant.
///
/// Every mutant ends up in exactly one of these states. The mutation score
/// (MSI) denominator is `Killed + Survived + Timeout` — the other variants
/// are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutantStatus {
    /// Test suite failed → mutation detected.
    Killed,
    /// Test suite passed → mutation undetected.
    Survived,
    /// Mutation caused an infinite loop or hang → detected via timeout.
    Timeout,
    /// TCE detected identical bytecode → unkillable, excluded from score.
    Equivalent,
    /// No test covers the mutated line → skipped, excluded from score.
    NotCovered,
    /// Mutation produces invalid Dart → skipped (should be <2%).
    CompileError,
}

impl MutantStatus {
    /// Returns true if this status counts toward the mutation score denominator.
    pub fn is_scored(&self) -> bool {
        matches!(
            self,
            MutantStatus::Killed | MutantStatus::Survived | MutantStatus::Timeout
        )
    }

    /// Returns true if this status counts as "killed" for scoring purposes.
    pub fn is_killed(&self) -> bool {
        matches!(self, MutantStatus::Killed | MutantStatus::Timeout)
    }

    /// String code used in reports and JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            MutantStatus::Killed => "KILLED",
            MutantStatus::Survived => "SURVIVED",
            MutantStatus::Timeout => "TIMEOUT",
            MutantStatus::Equivalent => "EQUIVALENT",
            MutantStatus::NotCovered => "NOT_COVERED",
            MutantStatus::CompileError => "COMPILE_ERROR",
        }
    }
}

impl fmt::Display for MutantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MutantStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "KILLED" => Ok(MutantStatus::Killed),
            "SURVIVED" => Ok(MutantStatus::Survived),
            "TIMEOUT" => Ok(MutantStatus::Timeout),
            "EQUIVALENT" => Ok(MutantStatus::Equivalent),
            "NOT_COVERED" => Ok(MutantStatus::NotCovered),
            "COMPILE_ERROR" => Ok(MutantStatus::CompileError),
            other => Err(format!("unknown mutant status: {}", other)),
        }
    }
}

/// A stable, human-readable identifier for a mutant.
///
/// Format: `{operator_code}:{file_index}:{line}:{column}` — but the id is
/// arbitrary as long as it's unique within a single mutation run. We generate
/// it as a monotonic integer string by default (`"0"`, `"1"`, …) since the
/// schemata switches on `DART_MUTANT_ID` which is set to this id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutantId(pub String);

impl MutantId {
    /// Create a new mutant id from a string.
    pub fn new(s: impl Into<String>) -> Self {
        MutantId(s.into())
    }

    /// Create a numeric mutant id.
    pub fn from_index(idx: usize) -> Self {
        MutantId(idx.to_string())
    }

    /// Get the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MutantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for MutantId {
    fn from(idx: usize) -> Self {
        MutantId::from_index(idx)
    }
}

impl From<String> for MutantId {
    fn from(s: String) -> Self {
        MutantId(s)
    }
}

/// A location in source code where a mutation can be applied.
///
/// Produced by a [`Mutator`](crate::operators::Mutator) during the discovery
/// phase. Contains the original text snippet and the byte range it occupies
/// in the source file, so the schemata generator can wrap it in a conditional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationPoint {
    /// 1-based line number in the source file.
    pub line: usize,
    /// 1-based column number (character offset from line start).
    pub column: usize,
    /// Inclusive start byte offset in the source text.
    pub start_byte: usize,
    /// Exclusive end byte offset in the source text.
    pub end_byte: usize,
    /// The original source text at this mutation point.
    pub original: String,
}

impl MutationPoint {
    /// Create a new mutation point.
    pub fn new(
        line: usize,
        column: usize,
        start_byte: usize,
        end_byte: usize,
        original: impl Into<String>,
    ) -> Self {
        MutationPoint {
            line,
            column,
            start_byte,
            end_byte,
            original: original.into(),
        }
    }

    /// The byte length of the original text.
    pub fn len(&self) -> usize {
        self.end_byte - self.start_byte
    }

    /// Whether the original text is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single mutation: a replacement of `original` text with `replacement` text
/// at a specific location in a source file.
///
/// Produced by [`Mutator::find_mutations`](crate::operators::Mutator::find_mutations).
/// Each mutant gets a unique [`MutantId`] assigned by the orchestrator after
/// collection (the operator itself doesn't assign the final id — it sets a
/// placeholder that the orchestrator rewrites).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutant {
    /// Unique identifier (assigned by orchestrator; may be empty during discovery).
    pub id: String,
    /// Path to the source file being mutated.
    pub file_path: String,
    /// 1-based line number of the mutation.
    pub line: usize,
    /// 1-based column number of the mutation.
    pub column: usize,
    /// Operator code (e.g. `\"AOR\"`, `\"ROR\"`, `\"NullSafety\"`).
    pub operator: String,
    /// Original source text being replaced.
    pub original: String,
    /// Replacement source text.
    pub replacement: String,
    /// Human-readable description of what this mutant does.
    pub description: String,
}

impl Mutant {
    /// Create a new mutant with all fields specified.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        file_path: impl Into<String>,
        line: usize,
        column: usize,
        operator: impl Into<String>,
        original: impl Into<String>,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Mutant {
            id: id.into(),
            file_path: file_path.into(),
            line,
            column,
            operator: operator.into(),
            original: original.into(),
            replacement: replacement.into(),
            description: description.into(),
        }
    }

    /// Create a mutant with a placeholder id (empty string). The orchestrator
    /// assigns the real id after collecting all mutants.
    pub fn without_id(
        file_path: impl Into<String>,
        line: usize,
        column: usize,
        operator: impl Into<String>,
        original: impl Into<String>,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Mutant::new(
            "",
            file_path,
            line,
            column,
            operator,
            original,
            replacement,
            description,
        )
    }

    /// Whether this mutant is equivalent to no-op (original == replacement).
    pub fn is_noop(&self) -> bool {
        self.original == self.replacement
    }

    /// A short label for reports: `{operator} at {file}:{line}`.
    pub fn label(&self) -> String {
        // Show just the file name, not the full path, for compactness.
        let short_path = self.file_path.rsplit('/').next().unwrap_or(&self.file_path);
        format!("{} @ {}:{}", self.operator, short_path, self.line)
    }
}

/// Result of running a mutant against the test suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantResult {
    /// The mutant that was tested.
    pub mutant: Mutant,
    /// The final status after running the test suite.
    pub status: MutantStatus,
    /// Names of tests that cover this mutant (for routing).
    #[serde(default)]
    pub covering_tests: Vec<String>,
    /// Optional diagnostic message (e.g. compiler error text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl MutantResult {
    /// Create a new result.
    pub fn new(mutant: Mutant, status: MutantStatus) -> Self {
        MutantResult {
            mutant,
            status,
            covering_tests: Vec::new(),
            message: None,
        }
    }

    /// Set the covering tests.
    pub fn with_tests(mut self, tests: Vec<String>) -> Self {
        self.covering_tests = tests;
        self
    }

    /// Set the diagnostic message.
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
}

/// Compute the mutation score (MSI) from a slice of results.
///
/// MSI = `KILLED / (KILLED + SURVIVED + TIMEOUT) × 100`
///
/// Returns 0.0 if no mutants were scored.
pub fn mutation_score(results: &[MutantResult]) -> f64 {
    let scored: Vec<_> = results.iter().filter(|r| r.status.is_scored()).collect();
    if scored.is_empty() {
        return 0.0;
    }
    let killed = scored.iter().filter(|r| r.status.is_killed()).count();
    killed as f64 / scored.len() as f64 * 100.0
}

/// Compute the mutation coverage from a slice of results.
///
/// Mutation Coverage = `(KILLED + SURVIVED) / (KILLED + SURVIVED + NOT_COVERED) × 100`
pub fn mutation_coverage(results: &[MutantResult]) -> f64 {
    let killed = results
        .iter()
        .filter(|r| r.status == MutantStatus::Killed)
        .count();
    let survived = results
        .iter()
        .filter(|r| r.status == MutantStatus::Survived)
        .count();
    let not_covered = results
        .iter()
        .filter(|r| r.status == MutantStatus::NotCovered)
        .count();
    let denom = killed + survived + not_covered;
    if denom == 0 {
        return 0.0;
    }
    (killed + survived) as f64 / denom as f64 * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serde() {
        let s = serde_json::to_string(&MutantStatus::Killed).unwrap();
        assert_eq!(s, "\"killed\"");
        let s2 = serde_json::to_string(&MutantStatus::CompileError).unwrap();
        assert_eq!(s2, "\"compile_error\"");
    }

    #[test]
    fn test_status_from_str() {
        assert_eq!(
            "killed".parse::<MutantStatus>().unwrap(),
            MutantStatus::Killed
        );
        assert_eq!(
            "SURVIVED".parse::<MutantStatus>().unwrap(),
            MutantStatus::Survived
        );
        assert!("bogus".parse::<MutantStatus>().is_err());
    }

    #[test]
    fn test_mutation_score() {
        let m = Mutant::without_id("f.dart", 1, 1, "AOR", "+", "-", "test");
        let results = vec![
            MutantResult::new(m.clone(), MutantStatus::Killed),
            MutantResult::new(m.clone(), MutantStatus::Survived),
            MutantResult::new(m.clone(), MutantStatus::Timeout),
            MutantResult::new(m.clone(), MutantStatus::Equivalent),
            MutantResult::new(m, MutantStatus::NotCovered),
        ];
        // killed (as in Killed only) = 1, but is_killed() counts Timeout too → 2
        // scored = 3 (Killed + Survived + Timeout)
        // MSI = is_killed / scored = 2/3 = 66.67%
        let score = mutation_score(&results);
        assert!((score - 66.66666).abs() < 0.01);
    }

    #[test]
    fn test_mutation_coverage() {
        let m = Mutant::without_id("f.dart", 1, 1, "AOR", "+", "-", "test");
        let results = vec![
            MutantResult::new(m.clone(), MutantStatus::Killed),
            MutantResult::new(m.clone(), MutantStatus::Survived),
            MutantResult::new(m, MutantStatus::NotCovered),
        ];
        // (1+1)/(1+1+1) = 66.67%
        let cov = mutation_coverage(&results);
        assert!((cov - 66.66666).abs() < 0.01);
    }

    #[test]
    fn test_mutant_noop() {
        let m = Mutant::without_id("f.dart", 1, 1, "AOR", "+", "+", "noop");
        assert!(m.is_noop());
        let m2 = Mutant::without_id("f.dart", 1, 1, "AOR", "+", "-", "real");
        assert!(!m2.is_noop());
    }
}
