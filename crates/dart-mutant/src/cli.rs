//! CLI argument definitions using clap derive.
//!
//! Every flag maps 1:1 to a field in [`crate::config::Config`] and is merged
//! with `.dart_mutant.yml` + defaults in [`crate::config`].

use clap::Parser;

/// dart_mutant — AST-based mutation testing for Dart and Flutter.
///
/// Injects deliberate faults (mutants) into Dart source code and runs the test
/// suite against each one. If the tests still pass, the mutant "survived" —
/// revealing a gap in test coverage.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "dart_mutant",
    version,
    about = "AST-based mutation testing for Dart and Flutter",
    long_about = "dart_mutant injects deliberate faults (mutants) into Dart source code and \
                  runs the test suite against each one. If the tests still pass, the mutant \
                  \"survived\" — revealing a gap in test coverage.\n\n\
                  Configuration is read from CLI flags (highest priority), then \
                  .dart_mutant.yml in the project root, then built-in defaults (lowest priority)."
)]
pub struct Cli {
    /// Path to the Dart/Flutter project root (must contain pubspec.yaml).
    #[arg(long, default_value = ".")]
    pub path: String,

    /// MSI threshold (0–100) for the CI gate. Exit 0 if MSI ≥ threshold, exit 1 if below.
    #[arg(long)]
    pub threshold: Option<f64>,

    /// Number of parallel workers for mutant execution. Defaults to CPU count.
    #[arg(long)]
    pub parallel: Option<usize>,

    /// Output format(s): comma-separated list of `console`, `json`, `html`, `junit`.
    /// Example: `--format html,json,junit`
    #[arg(long, value_delimiter = ',')]
    pub format: Option<Vec<String>>,

    /// Only mutate files changed since `base_ref` (incremental mode).
    #[arg(long)]
    pub incremental: bool,

    /// Git ref to diff against in incremental mode (e.g. `main`, `origin/main`).
    #[arg(long)]
    pub base_ref: Option<String>,

    /// Enable Trivial Compiler Equivalence (TCE) detection for equivalent mutants.
    #[arg(long)]
    pub detect_equivalent: bool,

    /// Restrict to specific mutation operators (comma-separated).
    /// Examples: `AOR,ROR,NullSafety,Cascade,AsyncAwait`
    #[arg(long, value_delimiter = ',')]
    pub operators: Option<Vec<String>>,

    /// Limit to N randomly sampled mutants (for quick feedback).
    #[arg(long)]
    pub sample: Option<usize>,

    /// Re-run a single mutant by its numeric ID.
    #[arg(long)]
    pub mutant: Option<u64>,

    /// Override the test command (defaults to `dart test` or `flutter test`).
    #[arg(long)]
    pub test_command: Option<String>,

    /// Comma-separated glob patterns to exclude from mutation.
    /// Example: `--exclude "lib/generated/**,lib/l10n/**,*.g.dart"`
    #[arg(long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,

    /// Quiet mode: suppress progress output. When `--format json` is set, stdout
    /// contains ONLY the JSON report (no other output).
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Disable ANSI color codes in console output.
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Adaptive timeout coefficient: per-mutant timeout = baseline × coefficient.
    /// Default: 3.0
    #[arg(long)]
    pub timeout_coefficient: Option<f64>,
    // --version is auto-provided by #[command(version, ...)] above.
}
