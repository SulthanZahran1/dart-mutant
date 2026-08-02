//! dart_mutant-runner — test execution, coverage routing, and parallel scheduling.
//!
//! This crate provides:
//! - [`coverage`] — runs `dart test --coverage` / `flutter test --coverage`, parses
//!   LCOV output, builds a per-test → per-line coverage map.
//! - [`timeout`] — adaptive per-mutant timeout based on baseline test duration × coefficient.
//! - [`cache`] — content-addressed cache using sha256 of (source bytes + test bytes).
//! - [`scheduler`] — parallel mutant execution using Rayon, routing each mutant only
//!   to covering tests.
//!
//! The runner takes a list of [`Mutant`]s and a [`RunnerConfig`] and produces
//! [`MutantResult`]s.

pub mod cache;
pub mod coverage;
pub mod scheduler;
pub mod timeout;

// Re-export the main types and functions for convenient access.
pub use cache::{Cache, CacheEntry};
pub use coverage::{build_coverage_map, collect_coverage, CoverageData, CoverageMap};
pub use scheduler::{run_mutants, run_mutants_parallel, RunStats};
pub use timeout::{AdaptiveTimeout, TimeoutCalculator};

use std::path::PathBuf;

use anyhow::Result;

use dart_mutant_core::{Mutant, MutantResult};

/// A compiled mutant schemata entry — the mutated source for a single mutant.
#[derive(Debug, Clone)]
pub struct SchemataEntry {
    /// The full mutated source code for the file containing this mutant.
    pub mutated_source: String,
    /// The file path relative to the project root.
    pub file_path: String,
}

/// A collection of all mutant schemata entries, keyed by mutant ID.
///
/// This is produced by [`dart_mutant_core::generate_schemata`] and consumed
/// by the scheduler to write mutated source files during test execution.
#[derive(Debug, Clone, Default)]
pub struct Schemata {
    /// Map of mutant ID → schemata entry (mutated source + file path).
    pub mutants: std::collections::HashMap<String, SchemataEntry>,
}

impl Schemata {
    /// Create an empty schemata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a schemata from a list of mutants and their source files.
    /// Generates the mutated source for each mutant using line-based replacement.
    pub fn from_mutants(
        mutants: &[Mutant],
        sources: &std::collections::HashMap<String, String>,
    ) -> Self {
        let mut map = std::collections::HashMap::new();
        for m in mutants {
            if let Some(source) = sources.get(&m.file_path) {
                let mutated = apply_mutation(source, m);
                map.insert(
                    m.id.clone(),
                    SchemataEntry {
                        mutated_source: mutated,
                        file_path: m.file_path.clone(),
                    },
                );
            }
        }
        Schemata { mutants: map }
    }

    /// Look up a mutant's schemata entry by ID.
    pub fn get(&self, id: &str) -> Option<&SchemataEntry> {
        self.mutants.get(id)
    }
}

/// Apply a single mutation to source code by replacing the original text
/// with the replacement text at the mutant's line.
fn apply_mutation(source: &str, mutant: &Mutant) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    if mutant.line == 0 || mutant.line > result.len() {
        return source.to_string();
    }

    let idx = mutant.line - 1;
    let line = &result[idx];

    // Replace the first occurrence of the original text with the replacement
    if mutant.original.is_empty() {
        // For SDL (statement deletion) — replace the whole line with empty
        result[idx] = String::new();
    } else if line.contains(&mutant.original) {
        result[idx] = line.replacen(&mutant.original, &mutant.replacement, 1);
    }

    result.join("\n")
}

// ---------------------------------------------------------------------------
// RunnerConfig
// ---------------------------------------------------------------------------

/// Configuration for the mutant runner.
///
/// Produced from the CLI [`Config`](../dart_mutant/config/struct.Config.html) by
/// extracting the fields the runner needs. The runner does not depend on the
/// CLI crate, so this is a standalone struct.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Absolute path to the Dart/Flutter project root.
    pub project_path: PathBuf,
    /// Test command to execute (e.g. `dart test`, `flutter test`).
    pub test_command: String,
    /// Number of parallel workers for mutant execution.
    pub parallel: usize,
    /// Adaptive timeout coefficient: per-mutant timeout = baseline × this.
    pub timeout_coefficient: f64,
    /// Minimum timeout in milliseconds (floor — never go below this).
    pub min_timeout_ms: u64,
    /// Maximum timeout in milliseconds (ceiling — never go above this).
    pub max_timeout_ms: u64,
    /// Directory for content-addressed cache (default: `.dart_mutant_cache`).
    pub cache_dir: PathBuf,
    /// Whether to use the cache (incremental mode).
    pub use_cache: bool,
    /// Coverage output directory (default: `coverage`).
    pub coverage_dir: String,
}

impl RunnerConfig {
    /// Create a new runner config with the given project path and test command.
    pub fn new(project_path: PathBuf, test_command: String) -> Self {
        let cache_dir = project_path.join(".dart_mutant_cache");
        RunnerConfig {
            project_path,
            test_command,
            parallel: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            timeout_coefficient: 3.0,
            min_timeout_ms: 5_000,   // 5 seconds minimum
            max_timeout_ms: 300_000, // 5 minutes maximum
            cache_dir,
            use_cache: true,
            coverage_dir: "coverage".to_string(),
        }
    }

    /// Set the parallelism level.
    pub fn with_parallel(mut self, parallel: usize) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the timeout coefficient.
    pub fn with_timeout_coefficient(mut self, coeff: f64) -> Self {
        self.timeout_coefficient = coeff;
        self
    }

    /// Disable the cache.
    pub fn without_cache(mut self) -> Self {
        self.use_cache = false;
        self
    }
}

impl Default for RunnerConfig {
    fn default() -> Self {
        RunnerConfig::new(PathBuf::from("."), "dart test".to_string())
    }
}

// ---------------------------------------------------------------------------
// Top-level runner entry point
// ---------------------------------------------------------------------------

/// Run the full mutation testing pipeline.
///
/// This is the high-level entry point that:
/// 1. Collects coverage (baseline test run with `--coverage`)
/// 2. Builds the per-test → per-line coverage map
/// 3. Runs all mutants in parallel with coverage routing + adaptive timeout
/// 4. Returns the results
///
/// # Arguments
/// - `mutants` — the list of mutants to test
/// - `schemata` — the compiled mutant schemata (all mutations in one compilation)
/// - `config` — the runner configuration
pub fn run(
    mutants: &[Mutant],
    schemata: &Schemata,
    config: &RunnerConfig,
) -> Result<Vec<MutantResult>> {
    log::info!(
        "Starting mutation testing run with {} mutants",
        mutants.len()
    );

    // Stage 1: Collect coverage + baseline timing
    log::info!("Collecting coverage with: {}", config.test_command);
    let coverage = coverage::collect_coverage(&config.project_path, &config.test_command)?;

    // Stage 2: Build per-mutant → covering-tests map
    log::info!("Building coverage map for {} mutants", mutants.len());
    let coverage_map = coverage::build_coverage_map(&coverage, mutants)?;

    // Stage 3: Initialize cache (if enabled)
    let mut cache = if config.use_cache {
        Some(cache::Cache::new(&config.cache_dir))
    } else {
        None
    };

    // Stage 4: Compute adaptive timeout
    let timeout_calc = timeout::TimeoutCalculator::new(
        coverage.baseline_duration_ms,
        config.timeout_coefficient,
        config.min_timeout_ms,
        config.max_timeout_ms,
    );
    log::info!(
        "Adaptive timeout: {}ms (baseline={}ms × {:.1})",
        timeout_calc.timeout_ms(),
        coverage.baseline_duration_ms,
        config.timeout_coefficient
    );

    // Stage 5: Run mutants in parallel
    let results = scheduler::run_mutants(
        mutants,
        schemata,
        &coverage_map,
        &timeout_calc,
        cache.as_mut(),
        config,
    )?;

    log::info!("Mutation testing complete: {} results", results.len());
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_config_default() {
        let config = RunnerConfig::default();
        assert_eq!(config.test_command, "dart test");
        assert!(config.parallel > 0);
        assert!((config.timeout_coefficient - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_runner_config_builder() {
        let config = RunnerConfig::new(PathBuf::from("/tmp/project"), "flutter test".to_string())
            .with_parallel(4)
            .with_timeout_coefficient(5.0)
            .without_cache();
        assert_eq!(config.project_path, PathBuf::from("/tmp/project"));
        assert_eq!(config.test_command, "flutter test");
        assert_eq!(config.parallel, 4);
        assert!((config.timeout_coefficient - 5.0).abs() < f64::EPSILON);
        assert!(!config.use_cache);
    }
}
