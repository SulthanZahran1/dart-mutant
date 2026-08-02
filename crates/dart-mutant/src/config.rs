//! Configuration management for dart_mutant.
//!
//! Merges CLI flags (highest priority) → `.dart_mutant.yml` (project root) →
//! built-in defaults (lowest priority) into a single [`Config`] struct.
//!
//! The [`Config`] struct is consumed by the pipeline in [`crate::main`] and by
//! the downstream libraries (`dart_mutant_core`, `_runner`, `_report`, `_tce`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

/// Default source directory inside a Dart project.
const DEFAULT_SOURCE_PATH: &str = "lib/";

/// Default glob patterns excluded from mutation.
const DEFAULT_EXCLUDES: &[&str] = &["*.g.dart", "*.freezed.dart", "*.mocks.dart"];

/// Default MSI threshold below which the CI gate fails (0 = always pass).
const DEFAULT_THRESHOLD: f64 = 0.0;

/// Default adaptive timeout multiplier.
const DEFAULT_TIMEOUT_COEFFICIENT: f64 = 3.0;

/// Default output format when none is specified.
const DEFAULT_FORMATS: &[&str] = &["console"];

/// All supported mutation operator codes.
pub const ALL_OPERATORS: &[&str] = &[
    // Generic operators (≥7)
    "AOR",
    "AOD",
    "AOI",
    "ROR",
    "LOR",
    "LCR",
    "COR",
    "SDL",
    "RVR",
    "INC",
    // Dart-specific operators (≥6 — the differentiator)
    "NullSafety",
    "NullAssert",
    "OptionalChaining",
    "Cascade",
    "AsyncAwait",
    "SealedExhaustiveness",
    "StreamMutation",
];

/// Available output formats.
#[allow(dead_code)]
pub const ALL_FORMATS: &[&str] = &["console", "json", "html", "junit"];

// ---------------------------------------------------------------------------
// Config file (`.dart_mutant.yml`) deserialization shape
// ---------------------------------------------------------------------------

/// Represents the `.dart_mutant.yml` file contents.
///
/// Every field is `Option<T>` so that missing keys fall through to CLI or
/// default values during the merge.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub test_command: Option<String>,
    pub source_path: Option<String>,
    pub exclude: Option<Vec<String>>,
    pub threshold: Option<f64>,
    pub parallel: Option<usize>,
    pub timeout_coefficient: Option<f64>,
    pub detect_equivalent: Option<bool>,
    pub incremental: Option<bool>,
    pub base_ref: Option<String>,
    pub format: Option<Vec<String>>,
    pub operators: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Resolved Config (the final merged struct used by the pipeline)
// ---------------------------------------------------------------------------

/// Fully-resolved configuration after merging CLI → config file → defaults.
///
/// This is the struct the pipeline consumes. There are no `Option`s left —
/// every field has a concrete value.
#[derive(Debug, Clone, Serialize)]
pub struct Config {
    /// Absolute path to the Dart/Flutter project root.
    pub path: PathBuf,

    /// Test command to execute (e.g. `dart test`, `flutter test`).
    pub test_command: String,

    /// Source directory relative to the project root (e.g. `lib/`).
    pub source_path: String,

    /// Glob patterns of files/dirs to exclude from mutation.
    pub exclude: Vec<String>,

    /// MSI threshold (0–100) for the CI gate.
    pub threshold: f64,

    /// Number of parallel workers.
    pub parallel: usize,

    /// Adaptive timeout coefficient: per-mutant timeout = baseline × this.
    pub timeout_coefficient: f64,

    /// Whether TCE equivalent-mutant detection is enabled.
    pub detect_equivalent: bool,

    /// Whether incremental mode is enabled (only changed files).
    pub incremental: bool,

    /// Git ref for incremental mode (e.g. `main`, `origin/main`).
    pub base_ref: Option<String>,

    /// Output formats to generate.
    pub formats: Vec<String>,

    /// Mutation operators to use (restricted subset of [`ALL_OPERATORS`]).
    pub operators: Vec<String>,

    /// If set, limit to N randomly sampled mutants.
    pub sample: Option<usize>,

    /// If set, re-run only the mutant with this numeric ID.
    pub mutant_id: Option<u64>,

    /// Quiet mode: suppress progress output.
    pub quiet: bool,

    /// Disable ANSI color in console output.
    pub no_color: bool,
}

// ---------------------------------------------------------------------------
// Merge logic
// ---------------------------------------------------------------------------

impl Config {
    /// Build the final [`Config`] from CLI args + `.dart_mutant.yml` + defaults.
    ///
    /// Priority: CLI flag (if `Some`) > config file value (if `Some`) > default.
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let project_path = canonicalize_project_path(&cli.path)?;

        // Load config file from the project root (may not exist → all defaults).
        let config_file = load_config_file(&project_path)?;

        // Merge: CLI > file > default for each field.
        let test_command = cli
            .test_command
            .clone()
            .or(config_file.test_command.clone())
            .unwrap_or_else(detect_test_command);

        let source_path = config_file
            .source_path
            .clone()
            .unwrap_or_else(|| DEFAULT_SOURCE_PATH.to_string());

        let exclude = cli
            .exclude
            .clone()
            .or(config_file.exclude.clone())
            .unwrap_or_else(|| DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect());

        let threshold = cli
            .threshold
            .or(config_file.threshold)
            .unwrap_or(DEFAULT_THRESHOLD);

        let parallel = cli
            .parallel
            .or(config_file.parallel)
            .unwrap_or_else(num_cpus_available);

        let timeout_coefficient = cli
            .timeout_coefficient
            .or(config_file.timeout_coefficient)
            .unwrap_or(DEFAULT_TIMEOUT_COEFFICIENT);

        let detect_equivalent =
            cli.detect_equivalent || config_file.detect_equivalent.unwrap_or(false);

        let incremental = cli.incremental || config_file.incremental.unwrap_or(false);

        let base_ref = cli
            .base_ref
            .clone()
            .or(config_file.base_ref.clone())
            .or(if incremental {
                Some("main".to_string())
            } else {
                None
            });

        let formats = cli
            .format
            .clone()
            .or(config_file.format.clone())
            .unwrap_or_else(|| DEFAULT_FORMATS.iter().map(|s| s.to_string()).collect());

        let operators = cli
            .operators
            .clone()
            .or(config_file.operators.clone())
            .unwrap_or_else(|| ALL_OPERATORS.iter().map(|s| s.to_string()).collect());

        let sample = cli.sample.or(None);
        let mutant_id = cli.mutant.or(None);
        let quiet = cli.quiet;
        let no_color = cli.no_color;

        let config = Config {
            path: project_path,
            test_command,
            source_path,
            exclude,
            threshold,
            parallel,
            timeout_coefficient,
            detect_equivalent,
            incremental,
            base_ref,
            formats,
            operators,
            sample,
            mutant_id,
            quiet,
            no_color,
        };

        Ok(config)
    }

    /// Returns `true` if JSON output is among the requested formats.
    pub fn wants_json(&self) -> bool {
        self.formats.iter().any(|f| f.eq_ignore_ascii_case("json"))
    }

    /// Returns `true` if console output is among the requested formats.
    pub fn wants_console(&self) -> bool {
        self.formats
            .iter()
            .any(|f| f.eq_ignore_ascii_case("console"))
    }

    /// Returns `true` if HTML output is among the requested formats.
    pub fn wants_html(&self) -> bool {
        self.formats.iter().any(|f| f.eq_ignore_ascii_case("html"))
    }

    /// Returns `true` if JUnit XML output is among the requested formats.
    pub fn wants_junit(&self) -> bool {
        self.formats.iter().any(|f| f.eq_ignore_ascii_case("junit"))
    }

    /// Suppress all non-JSON output when `--quiet` + `--format json` are set.
    pub fn suppress_progress(&self) -> bool {
        self.quiet && self.wants_json()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Canonicalize the user-supplied project path to an absolute [`PathBuf`].
///
/// Returns an error if the path does not exist or is not a directory.
fn canonicalize_project_path(raw: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let canonical = abs
        .canonicalize()
        .with_context(|| format!("project path '{}' does not exist", abs.display()))?;

    if !canonical.is_dir() {
        anyhow::bail!("project path '{}' is not a directory", canonical.display());
    }

    Ok(canonical)
}

/// Load `.dart_mutant.yml` from the project root if present.
///
/// Returns an empty [`ConfigFile`] (all `None`) if the file does not exist.
/// Returns an error if the file exists but cannot be parsed.
fn load_config_file(project_path: &Path) -> Result<ConfigFile> {
    let config_path = project_path.join(".dart_mutant.yml");

    if !config_path.exists() {
        return Ok(ConfigFile::default());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;

    let parsed: ConfigFile = serde_yaml::from_str(&contents).with_context(|| {
        format!(
            "failed to parse YAML config file: {}",
            config_path.display()
        )
    })?;

    Ok(parsed)
}

/// Detect the test command to use.
///
/// Defaults to `flutter test` if a `pubspec.yaml` with Flutter SDK dependency
/// is found and `flutter` is on `PATH`; otherwise `dart test`.
fn detect_test_command() -> String {
    // Heuristic: check if `flutter` is on PATH and pubspec has flutter dependency.
    if which("flutter").is_some() && has_flutter_dep() {
        "flutter test".to_string()
    } else {
        "dart test".to_string()
    }
}

/// Returns `true` if the current directory's `pubspec.yaml` contains a
/// Flutter SDK dependency.
fn has_flutter_dep() -> bool {
    let pubspec = Path::new("pubspec.yaml");
    if !pubspec.exists() {
        return false;
    }
    match std::fs::read_to_string(pubspec) {
        Ok(contents) => contents.contains("flutter") && contents.contains("sdk: flutter"),
        Err(_) => false,
    }
}

/// Get the number of available CPUs (fallback: 4).
fn num_cpus_available() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Minimal `which` implementation — searches `PATH` for the given executable.
fn which(cmd: &str) -> Option<PathBuf> {
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_all_operators_count() {
        // ≥6 Dart-specific + ≥7 generic = ≥13 total
        assert!(ALL_OPERATORS.len() >= 13);
    }

    #[test]
    fn test_default_threshold() {
        assert_eq!(DEFAULT_THRESHOLD, 0.0);
    }

    #[test]
    fn test_default_timeout_coefficient() {
        assert_eq!(DEFAULT_TIMEOUT_COEFFICIENT, 3.0);
    }

    #[test]
    fn test_cli_parse_threshold() {
        let cli = Cli::try_parse_from(["dart_mutant", "--threshold", "80"]).unwrap();
        assert_eq!(cli.threshold, Some(80.0));
    }

    #[test]
    fn test_cli_parse_formats() {
        let cli = Cli::try_parse_from(["dart_mutant", "--format", "html,json,junit"]).unwrap();
        let formats = cli.format.unwrap();
        assert_eq!(formats, vec!["html", "json", "junit"]);
    }

    #[test]
    fn test_cli_parse_operators() {
        let cli = Cli::try_parse_from(["dart_mutant", "--operators", "AOR,ROR,Cascade"]).unwrap();
        let ops = cli.operators.unwrap();
        assert_eq!(ops, vec!["AOR", "ROR", "Cascade"]);
    }

    #[test]
    fn test_cli_parse_exclude() {
        let cli =
            Cli::try_parse_from(["dart_mutant", "--exclude", "*.g.dart,lib/generated/**"]).unwrap();
        let excl = cli.exclude.unwrap();
        assert_eq!(excl, vec!["*.g.dart", "lib/generated/**"]);
    }

    #[test]
    fn test_cli_parse_mutant() {
        let cli = Cli::try_parse_from(["dart_mutant", "--mutant", "42"]).unwrap();
        assert_eq!(cli.mutant, Some(42));
    }

    #[test]
    fn test_cli_parse_incremental() {
        let cli =
            Cli::try_parse_from(["dart_mutant", "--incremental", "--base-ref", "develop"]).unwrap();
        assert!(cli.incremental);
        assert_eq!(cli.base_ref.as_deref(), Some("develop"));
    }

    #[test]
    fn test_cli_default_path() {
        let cli = Cli::try_parse_from(["dart_mutant"]).unwrap();
        assert_eq!(cli.path, ".");
    }

    #[test]
    fn test_config_file_default_is_none() {
        let cf = ConfigFile::default();
        assert!(cf.test_command.is_none());
        assert!(cf.threshold.is_none());
    }

    #[test]
    fn test_config_file_parse() {
        let yaml = r#"
test_command: "flutter test"
source_path: "lib/"
threshold: 80
parallel: 8
timeout_coefficient: 3.0
detect_equivalent: false
incremental: false
base_ref: "main"
format:
  - console
  - html
  - json
operators:
  - AOR
  - ROR
  - Cascade
"#;
        let cf: ConfigFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cf.test_command.as_deref(), Some("flutter test"));
        assert_eq!(cf.threshold, Some(80.0));
        assert_eq!(cf.parallel, Some(8));
        assert_eq!(cf.operators.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_wants_json_detection() {
        let mut cli = Cli::try_parse_from(["dart_mutant", "--format", "json,html"]).unwrap();
        let config = Config::from_cli(&cli).unwrap();
        assert!(config.wants_json());
        assert!(config.wants_html());
        assert!(config.wants_console() == false);
    }
}
