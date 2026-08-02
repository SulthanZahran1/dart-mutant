//! Terminal summary with colored output.
//!
//! Produces a human-readable console summary including MSI, killed/survived/
//! timeout/equivalent/not_covered/compile_error counts, and a pass/fail
//! indicator relative to a threshold.
//!
//! # Colors
//!
//! ANSI escape codes are used for coloring. When `use_color` is `false`, all
//! ANSI codes are suppressed — useful for CI pipelines that capture stdout
//! or for the `--no-color` CLI flag.
//!
//! # Example
//!
//! ```text
//! dart_mutant v0.1.0 — Mutation Testing for Dart
//!
//! Results:
//!   Killed:        689  (55.3%)
//!   Survived:      158  (12.7%)
//!   Timeout:        12  (1.0%)
//!   Equivalent:     43  (excluded)
//!   Not covered:   298  (excluded)
//!   Compile error:   7  (0.6%)
//!
//!   Mutation Score (MSI): 80.2%
//!   Threshold: 80%       ✅ PASSED
//! ```

use crate::{count_by_status, mutation_coverage, mutation_score, MutantResult};

// ---------------------------------------------------------------------------
// ANSI color codes
// ---------------------------------------------------------------------------

struct Colors {
    enabled: bool,
}

impl Colors {
    const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("{}{}{}", code, text, "\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn green(&self, text: &str) -> String {
        self.paint("\x1b[32m", text)
    }
    fn red(&self, text: &str) -> String {
        self.paint("\x1b[31m", text)
    }
    fn yellow(&self, text: &str) -> String {
        self.paint("\x1b[33m", text)
    }
    fn orange(&self, text: &str) -> String {
        self.paint("\x1b[38;5;208m", text)
    }
    fn gray(&self, text: &str) -> String {
        self.paint("\x1b[90m", text)
    }
    fn blue(&self, text: &str) -> String {
        self.paint("\x1b[34m", text)
    }
    fn purple(&self, text: &str) -> String {
        self.paint("\x1b[35m", text)
    }
    fn bold(&self, text: &str) -> String {
        if self.enabled {
            format!("\x1b[1m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }
    fn cyan(&self, text: &str) -> String {
        self.paint("\x1b[36m", text)
    }
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Options for console report generation.
#[derive(Debug, Clone)]
pub struct ConsoleOptions {
    /// MSI threshold percentage (0-100). If MSI < threshold, the report shows "FAILED".
    /// Default: 80.0
    pub threshold: f64,
    /// Whether to use ANSI color codes. Default: true
    pub use_color: bool,
    /// Version string to display in the header. Default: `"0.1.0"`
    pub version: String,
}

impl Default for ConsoleOptions {
    fn default() -> Self {
        Self {
            threshold: 80.0,
            use_color: true,
            version: "0.1.0".to_string(),
        }
    }
}

/// Generate a console summary string from mutation results.
///
/// Uses default [`ConsoleOptions`] (threshold=80, color=true, version="0.1.0").
pub fn generate(results: &[MutantResult]) -> String {
    generate_with_options(results, &ConsoleOptions::default())
}

/// Generate a console summary string with custom options.
pub fn generate_with_options(results: &[MutantResult], opts: &ConsoleOptions) -> String {
    let c = Colors::new(opts.use_color);
    let counts = count_by_status(results);
    let msi = mutation_score(results);
    let coverage = mutation_coverage(results);

    let total = counts.total;
    let pct = |n: usize| -> String {
        if total == 0 {
            "0.0%".to_string()
        } else {
            format!("{:.1}%", n as f64 / total as f64 * 100.0)
        }
    };

    let mut out = String::with_capacity(2048);

    // Header
    out.push_str(&format!(
        "\n{} v{} — Mutation Testing for Dart\n\n",
        c.bold("dart_mutant"),
        opts.version
    ));

    // Results
    out.push_str(&c.bold("Results:\n"));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n",
        "Killed:",
        c.green(&counts.killed.to_string()),
        c.green(&pct(counts.killed))
    ));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n",
        "Survived:",
        c.red(&counts.survived.to_string()),
        c.red(&pct(counts.survived))
    ));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n",
        "Timeout:",
        c.orange(&counts.timeout.to_string()),
        c.orange(&pct(counts.timeout))
    ));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n",
        "Equivalent:",
        c.gray(&counts.equivalent.to_string()),
        c.gray("excluded")
    ));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n",
        "Not covered:",
        c.blue(&counts.not_covered.to_string()),
        c.gray("excluded")
    ));
    out.push_str(&format!(
        "  {:<14} {}  ({})\n\n",
        "Compile error:",
        c.purple(&counts.compile_error.to_string()),
        c.purple(&pct(counts.compile_error))
    ));

    // Scores
    let msi_colored = if msi >= opts.threshold {
        c.green(&format!("{:.1}%", msi))
    } else if msi >= 50.0 {
        c.yellow(&format!("{:.1}%", msi))
    } else {
        c.red(&format!("{:.1}%", msi))
    };

    out.push_str(&format!(
        "  {:<22} {}\n",
        c.bold("Mutation Score (MSI):"),
        msi_colored
    ));
    out.push_str(&format!(
        "  {:<22} {:.1}%\n\n",
        c.bold("Mutation Coverage:"),
        coverage
    ));

    // Threshold gate
    if msi >= opts.threshold {
        out.push_str(&format!(
            "  {:<22} {} {}\n",
            c.bold("Threshold:"),
            c.cyan(&format!("{}%", opts.threshold as u32)),
            c.green("✅ PASSED")
        ));
    } else {
        out.push_str(&format!(
            "  {:<22} {} {}\n",
            c.bold("Threshold:"),
            c.cyan(&format!("{}%", opts.threshold as u32)),
            c.red("❌ FAILED")
        ));
    }

    out
}

/// Generate a console summary and print it to stdout.
pub fn print(results: &[MutantResult]) {
    print!("{}", generate(results));
}

/// Generate a console summary with options and print it to stdout.
pub fn print_with_options(results: &[MutantResult], opts: &ConsoleOptions) {
    print!("{}", generate_with_options(results, opts));
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
    fn test_generates_summary() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::Timeout),
        ];
        let summary = generate(&results);
        assert!(summary.contains("dart_mutant"));
        assert!(summary.contains("Killed"));
        assert!(summary.contains("Survived"));
        assert!(summary.contains("Timeout"));
        assert!(summary.contains("Mutation Score (MSI)"));
    }

    #[test]
    fn test_msi_calculation() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::Equivalent),
        ];
        let summary = generate(&results);
        // MSI = 1 / (1+1) * 100 = 50%
        assert!(summary.contains("50.0%"));
    }

    #[test]
    fn test_threshold_passed() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Killed),
        ];
        let opts = ConsoleOptions {
            threshold: 80.0,
            use_color: false,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        assert!(summary.contains("PASSED"));
    }

    #[test]
    fn test_threshold_failed() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
        ];
        let opts = ConsoleOptions {
            threshold: 80.0,
            use_color: false,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        assert!(summary.contains("FAILED"));
    }

    #[test]
    fn test_no_color() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let opts = ConsoleOptions {
            use_color: false,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        assert!(!summary.contains("\x1b["));
    }

    #[test]
    fn test_with_color() {
        let results = vec![make_result("0", MutantStatus::Killed)];
        let opts = ConsoleOptions {
            use_color: true,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        assert!(summary.contains("\x1b["));
    }

    #[test]
    fn test_all_status_counts() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::Timeout),
            make_result("3", MutantStatus::Equivalent),
            make_result("4", MutantStatus::NotCovered),
            make_result("5", MutantStatus::CompileError),
        ];
        let opts = ConsoleOptions {
            use_color: false,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        assert!(summary.contains("Killed:"));
        assert!(summary.contains("Survived:"));
        assert!(summary.contains("Timeout:"));
        assert!(summary.contains("Equivalent:"));
        assert!(summary.contains("Not covered:"));
        assert!(summary.contains("Compile error:"));
    }

    #[test]
    fn test_empty_results() {
        let summary: String = generate(&[]);
        assert!(summary.contains("Mutation Score (MSI)"));
        assert!(summary.contains("0.0%"));
    }

    #[test]
    fn test_mutation_coverage_displayed() {
        let results = vec![
            make_result("0", MutantStatus::Killed),
            make_result("1", MutantStatus::Survived),
            make_result("2", MutantStatus::NotCovered),
        ];
        let opts = ConsoleOptions {
            use_color: false,
            ..Default::default()
        };
        let summary = generate_with_options(&results, &opts);
        // Coverage = (1+1)/(1+1+1) * 100 = 66.7%
        assert!(summary.contains("Mutation Coverage:"));
        assert!(summary.contains("66.7%"));
    }
}
