//! Report generation for dart_mutant.
//!
//! This crate provides four report formats from a single `&[MutantResult]`:
//!
//! - **Stryker JSON** — compatible with the [mutation-testing-elements](https://github.com/stryker-mutator/mutation-testing-elements) schema and Stryker dashboard.
//! - **JUnit XML** — standard JUnit format for CI test result panels (GitHub Actions, GitLab CI, Jenkins).
//! - **HTML** — self-contained HTML report with inline CSS, per-file mutation heatmap, and per-mutant detail.
//! - **Console** — colored terminal summary with MSI, killed/survived/timeout counts.
//!
//! # Usage
//!
//! ```no_run
//! use dart_mutant_core::{Mutant, MutantResult, MutantStatus};
//! use dart_mutant_report;
//!
//! let results: Vec<MutantResult> = vec![]; // your results here
//!
//! let json = dart_mutant_report::stryker_json::generate(&results).unwrap();
//! let xml  = dart_mutant_report::junit_xml::generate(&results).unwrap();
//! let html = dart_mutant_report::html::generate(&results).unwrap();
//! let summary = dart_mutant_report::console::generate(&results);
//! ```

pub mod console;
pub mod html;
pub mod junit_xml;
pub mod stryker_json;

// Re-export key types from core for convenience
pub use dart_mutant_core::{mutation_coverage, mutation_score, Mutant, MutantResult, MutantStatus};

// ---------------------------------------------------------------------------
// Local helpers (not in core)
// ---------------------------------------------------------------------------

/// Aggregated counts of mutants in each status category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusCounts {
    pub killed: usize,
    pub survived: usize,
    pub timeout: usize,
    pub equivalent: usize,
    pub not_covered: usize,
    pub compile_error: usize,
    pub total: usize,
}

/// Count mutants by status, returning a [`StatusCounts`] struct.
pub fn count_by_status(results: &[MutantResult]) -> StatusCounts {
    let mut counts = StatusCounts::default();
    for r in results {
        match r.status {
            MutantStatus::Killed => counts.killed += 1,
            MutantStatus::Survived => counts.survived += 1,
            MutantStatus::Timeout => counts.timeout += 1,
            MutantStatus::Equivalent => counts.equivalent += 1,
            MutantStatus::NotCovered => counts.not_covered += 1,
            MutantStatus::CompileError => counts.compile_error += 1,
        }
    }
    counts.total = results.len();
    counts
}

/// Convert a [`MutantStatus`] to the Stryker mutation-testing-elements status string.
///
/// Stryker uses: `Killed`, `Survived`, `Timeout`, `NoCoverage`, `CompileError`.
/// There is no standard Stryker status for "Equivalent" — we map it to
/// `Survived` since the Stryker schema doesn't have an equivalent variant.
pub fn stryker_status(status: MutantStatus) -> &'static str {
    match status {
        MutantStatus::Killed => "Killed",
        MutantStatus::Survived => "Survived",
        MutantStatus::Timeout => "Timeout",
        MutantStatus::Equivalent => "Survived",
        MutantStatus::NotCovered => "NoCoverage",
        MutantStatus::CompileError => "CompileError",
    }
}

/// Convert a [`MutantStatus`] to a lowercase kebab-case CSS class name.
pub fn status_css_class(status: MutantStatus) -> &'static str {
    match status {
        MutantStatus::Killed => "killed",
        MutantStatus::Survived => "survived",
        MutantStatus::Timeout => "timeout",
        MutantStatus::Equivalent => "equivalent",
        MutantStatus::NotCovered => "not-covered",
        MutantStatus::CompileError => "compile-error",
    }
}

/// Convert a [`MutantStatus`] to a human-readable display string.
pub fn status_display(status: MutantStatus) -> &'static str {
    match status {
        MutantStatus::Killed => "Killed",
        MutantStatus::Survived => "Survived",
        MutantStatus::Timeout => "Timeout",
        MutantStatus::Equivalent => "Equivalent",
        MutantStatus::NotCovered => "Not covered",
        MutantStatus::CompileError => "Compile error",
    }
}

/// Write a report string to a file, creating parent directories if needed.
///
/// Convenience wrapper around [`std::fs::write`] that also calls
/// [`std::fs::create_dir_all`] on the parent directory.
pub fn write_report_to_file(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)?;
    log::info!("Report written to {}", path.display());
    Ok(())
}
