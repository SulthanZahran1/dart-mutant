//! Parallel mutant execution using Rayon.
//!
//! The scheduler runs each mutant against the test suite in parallel, routing
//! each mutant only to the tests that cover the mutated line (coverage routing).
//! Each mutant gets an adaptive timeout based on the baseline test duration.
//!
//! # Execution model
//!
//! 1. The schemata contains the mutated source for each mutant.
//! 2. For each mutant, the scheduler writes the mutated source to the file
//!    system (swapping the original), sets `DART_MUTANT_ID=<id>`, and runs
//!    only the covering tests.
//! 3. If the test suite fails → `KILLED`. If it passes → `SURVIVED`.
//!    If it times out → `TIMEOUT`. If no tests cover the mutant → `NOT_COVERED`.
//! 4. The original source is restored after each mutant run.
//!
//! # Parallelism
//!
//! Mutants are run in parallel using Rayon. Since each mutant modifies a file
//! on disk, we need to be careful about file contention. The current approach
//! groups mutants by file and runs each file's mutants sequentially (since they
//! all modify the same file), while different files run in parallel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use log::{debug, info, warn};
use rayon::prelude::*;

use dart_mutant_core::{Mutant, MutantResult, MutantStatus};

use crate::cache::Cache;
use crate::coverage::CoverageMap;
use crate::timeout::TimeoutCalculator;
use crate::{RunnerConfig, Schemata};

// ---------------------------------------------------------------------------
// Run statistics
// ---------------------------------------------------------------------------

/// Statistics from a mutation testing run.
#[derive(Debug, Clone, Default)]
pub struct RunStats {
    /// Total number of mutants processed.
    pub total: usize,
    /// Number of mutants killed by tests.
    pub killed: usize,
    /// Number of mutants that survived (tests passed).
    pub survived: usize,
    /// Number of mutants that timed out.
    pub timeout: usize,
    /// Number of mutants with no test coverage.
    pub not_covered: usize,
    /// Number of mutants that had compile errors.
    pub compile_errors: usize,
    /// Number of mutants served from cache.
    pub from_cache: usize,
    /// Total wall-clock time in milliseconds.
    pub total_duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Scheduler entry point
// ---------------------------------------------------------------------------

/// Run all mutants in parallel against the schemata, routing each mutant
/// only to its covering tests.
///
/// This is the main entry point for mutant execution. It uses Rayon for
/// parallelism, with mutants grouped by source file to avoid file contention.
///
/// # Arguments
/// - `mutants` — the list of mutants to test
/// - `schemata` — the compiled mutant schemata containing mutated source
/// - `coverage_map` — per-mutant → covering tests mapping
/// - `timeout_calc` — adaptive timeout calculator
/// - `cache` — optional content-addressed cache
/// - `config` — runner configuration
pub fn run_mutants(
    mutants: &[Mutant],
    schemata: &Schemata,
    coverage_map: &CoverageMap,
    timeout_calc: &TimeoutCalculator,
    cache: Option<&Cache>,
    config: &RunnerConfig,
) -> Result<Vec<MutantResult>> {
    let start = Instant::now();
    let timeout_ms = timeout_calc.timeout_ms();

    info!(
        "Running {} mutants ({} parallel, timeout={}ms)",
        mutants.len(),
        config.parallel,
        timeout_ms
    );

    // Group mutants by file to avoid file contention in parallel execution.
    // Mutants on the same file run sequentially; different files run in parallel.
    let file_groups = group_mutants_by_file(mutants);

    // Configure Rayon thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.parallel)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create Rayon thread pool: {}", e))?;

    // Use a mutex for cache writes (cache is shared across threads)
    // Note: we pass cache as immutable reference; stores happen after the parallel run
    let _cache_hits: Arc<Mutex<Vec<(usize, crate::cache::CacheEntry)>>> =
        Arc::new(Mutex::new(Vec::new()));

    let results: Vec<MutantResult> = pool.install(|| {
        file_groups
            .par_iter()
            .flat_map(|(_file, file_mutants)| {
                // Each file's mutants run sequentially (they modify the same file)
                file_mutants
                    .iter()
                    .map(|mutant| {
                        run_single_mutant(mutant, schemata, coverage_map, timeout_ms, cache, config)
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    });

    let total_duration_ms = start.elapsed().as_millis() as u64;

    // Log statistics
    let stats = compute_stats(&results, total_duration_ms);
    info!(
        "Mutation testing complete: {} total, {} killed, {} survived, {} timeout, {} not_covered, {} compile_error ({}ms)",
        stats.total,
        stats.killed,
        stats.survived,
        stats.timeout,
        stats.not_covered,
        stats.compile_errors,
        stats.total_duration_ms
    );

    Ok(results)
}

/// Backward-compatible alias matching the old API signature.
pub fn run_mutants_parallel(
    schemata: &Schemata,
    coverage_map: &CoverageMap,
    parallel: usize,
    timeout_coefficient: f64,
    test_command: &str,
) -> Result<Vec<MutantResult>> {
    let config = RunnerConfig {
        project_path: PathBuf::from("."),
        test_command: test_command.to_string(),
        parallel,
        timeout_coefficient,
        min_timeout_ms: 5_000,
        max_timeout_ms: 300_000,
        cache_dir: PathBuf::from(".dart_mutant_cache"),
        use_cache: false,
        coverage_dir: "coverage".to_string(),
    };
    let timeout_calc = TimeoutCalculator::new(0, timeout_coefficient, 5_000, 300_000);
    // Collect all mutant IDs from the schemata
    let mutants: Vec<Mutant> = schemata
        .mutants
        .keys()
        .map(|id| Mutant::new(id, "", 0, 0, "", "", "", ""))
        .collect();
    run_mutants(
        &mutants,
        schemata,
        coverage_map,
        &timeout_calc,
        None,
        &config,
    )
}

// ---------------------------------------------------------------------------
// Single mutant execution
// ---------------------------------------------------------------------------

/// Run a single mutant against its covering tests.
///
/// This function:
/// 1. Checks if the mutant has any covering tests → if not, returns `NOT_COVERED`
/// 2. Looks up the mutant in the schemata to get the mutated source
/// 3. Writes the mutated source to the file (backing up the original)
/// 4. Runs the test suite with `DART_MUTANT_ID=<id>` env var
/// 5. Classifies the result: KILLED (test failed), SURVIVED (test passed), TIMEOUT
/// 6. Restores the original source file
fn run_single_mutant(
    mutant: &Mutant,
    schemata: &Schemata,
    coverage_map: &CoverageMap,
    timeout_ms: u64,
    _cache: Option<&Cache>,
    config: &RunnerConfig,
) -> MutantResult {
    let covering_tests = coverage_map.covering_tests(&mutant.id).to_vec();

    // No covering tests → NOT_COVERED
    if covering_tests.is_empty() {
        debug!("Mutant {}: NOT_COVERED (no covering tests)", mutant.id);
        return MutantResult::new(mutant.clone(), MutantStatus::NotCovered);
    }

    // Get the mutated source from the schemata
    let entry = match schemata.get(&mutant.id) {
        Some(e) => e,
        None => {
            warn!("Mutant {} not found in schemata — skipping", mutant.id);
            return MutantResult::new(mutant.clone(), MutantStatus::CompileError)
                .with_message("mutant not found in schemata");
        }
    };

    let file_path = config.project_path.join(&mutant.file_path);

    // Write the mutated source to the file system
    // Read original first
    let original_source = match std::fs::read_to_string(&file_path) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                "Mutant {}: failed to read {}: {}",
                mutant.id,
                file_path.display(),
                e
            );
            return MutantResult::new(mutant.clone(), MutantStatus::CompileError)
                .with_message(format!("failed to read source file: {e}"));
        }
    };

    // Write mutated source
    if let Err(e) = std::fs::write(&file_path, &entry.mutated_source) {
        warn!(
            "Mutant {}: failed to write mutated source: {}",
            mutant.id, e
        );
        return MutantResult::new(mutant.clone(), MutantStatus::CompileError)
            .with_message(format!("failed to write mutated source: {e}"));
    }

    // Run the test suite with DART_MUTANT_ID set
    let start = Instant::now();
    let status = run_test_suite(
        &config.test_command,
        &mutant.id,
        &covering_tests,
        &config.project_path,
        timeout_ms,
    );
    let duration_ms = start.elapsed().as_millis() as u64;

    // Restore original source immediately
    if let Err(e) = std::fs::write(&file_path, &original_source) {
        warn!(
            "Mutant {}: CRITICAL — failed to restore original source {}: {}",
            mutant.id,
            file_path.display(),
            e
        );
    }

    // Classify the result
    let result = match status {
        TestStatus::Passed => {
            debug!("Mutant {}: SURVIVED ({}ms)", mutant.id, duration_ms);
            MutantResult::new(mutant.clone(), MutantStatus::Survived).with_tests(covering_tests)
        }
        TestStatus::Failed => {
            debug!("Mutant {}: KILLED ({}ms)", mutant.id, duration_ms);
            MutantResult::new(mutant.clone(), MutantStatus::Killed).with_tests(covering_tests)
        }
        TestStatus::Timeout => {
            debug!("Mutant {}: TIMEOUT ({}ms)", mutant.id, duration_ms);
            MutantResult::new(mutant.clone(), MutantStatus::Timeout)
                .with_tests(covering_tests)
                .with_message(format!("exceeded {}ms timeout", timeout_ms))
        }
        TestStatus::CompileError(msg) => {
            debug!(
                "Mutant {}: COMPILE_ERROR ({}ms): {}",
                mutant.id, duration_ms, msg
            );
            MutantResult::new(mutant.clone(), MutantStatus::CompileError)
                .with_tests(covering_tests)
                .with_message(msg)
        }
        TestStatus::Error(msg) => {
            warn!("Mutant {}: ERROR ({}ms): {}", mutant.id, duration_ms, msg);
            MutantResult::new(mutant.clone(), MutantStatus::CompileError)
                .with_tests(covering_tests)
                .with_message(msg)
        }
    };

    result
}

// ---------------------------------------------------------------------------
// Test suite execution
// ---------------------------------------------------------------------------

/// Result of running the test suite for a single mutant.
#[derive(Debug, Clone)]
enum TestStatus {
    /// Tests passed (mutant survived).
    Passed,
    /// Tests failed (mutant killed).
    Failed,
    /// Tests timed out.
    Timeout,
    /// Compilation error.
    CompileError(String),
    /// Infrastructure error.
    Error(String),
}

/// Run the test suite for a single mutant.
///
/// Sets `DART_MUTANT_ID=<id>` environment variable and runs the configured
/// test command. If `covering_tests` is non-empty, passes `--plain-name` or
/// test name filters to run only the covering tests.
fn run_test_suite(
    test_command: &str,
    mutant_id: &str,
    covering_tests: &[String],
    project_path: &Path,
    timeout_ms: u64,
) -> TestStatus {
    let parts: Vec<&str> = test_command.split_whitespace().collect();
    if parts.is_empty() {
        return TestStatus::Error("empty test command".to_string());
    }

    let program = parts[0];
    let mut args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Coverage routing: run only the tests that cover the mutated line.
    // Dart's coverage JSON is keyed per TEST FILE (e.g. `math_utils_test.dart`),
    // so covering tests are file names — pass them as positional args to
    // `dart test test/foo_test.dart`. If a name is a test *description*
    // (not a path), fall back to `--plain-name`.
    if !covering_tests.is_empty() {
        let file_like = covering_tests
            .iter()
            .all(|t| t.ends_with(".dart") || t.ends_with(".vm.json"));
        if file_like {
            // Strip `.vm.json` suffix if present; pass paths as-is.
            for test in covering_tests {
                let test = test.trim_end_matches(".vm.json");
                args.push(test.to_string());
            }
        } else {
            for test_name in covering_tests {
                args.push("--plain-name".to_string());
                args.push(test_name.clone());
            }
        }
    }

    debug!(
        "Running: {} {} (DART_MUTANT_ID={})",
        program,
        args.join(" "),
        mutant_id
    );

    let mut cmd = Command::new(program);
    cmd.args(&args)
        .current_dir(project_path)
        .env("DART_MUTANT_ID", mutant_id)
        // Redirect child output so it never corrupts the parent's stdout
        // (critical for `--format json` where stdout must be pure JSON).
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return TestStatus::Error(format!(
                "failed to spawn `{} {}`: {}",
                program,
                args.join(" "),
                e
            ));
        }
    };

    // Wait with timeout
    let timeout = std::time::Duration::from_millis(timeout_ms);
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => {
            if status.success() {
                TestStatus::Passed
            } else {
                // Check if it was a compilation error
                let exit_code = status.code().unwrap_or(-1);
                if exit_code == 254 || exit_code == 255 {
                    // Dart compilation error codes
                    TestStatus::CompileError(format!("exit code {}", exit_code))
                } else {
                    TestStatus::Failed
                }
            }
        }
        Ok(None) => {
            // Timed out — kill the process
            let _ = child.kill();
            let _ = child.wait();
            TestStatus::Timeout
        }
        Err(e) => TestStatus::Error(format!("failed to wait for process: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Group mutants by their source file path.
///
/// Returns a Vec of (file_path, mutants_on_that_file) pairs. Each group
/// runs sequentially, but different groups run in parallel.
fn group_mutants_by_file(mutants: &[Mutant]) -> Vec<(String, Vec<&Mutant>)> {
    let mut groups: HashMap<String, Vec<&Mutant>> = HashMap::new();
    for m in mutants {
        groups.entry(m.file_path.clone()).or_default().push(m);
    }
    groups.into_iter().collect()
}

/// Compute run statistics from results.
fn compute_stats(results: &[MutantResult], total_duration_ms: u64) -> RunStats {
    let mut stats = RunStats {
        total: results.len(),
        total_duration_ms,
        ..Default::default()
    };

    for r in results {
        match r.status {
            MutantStatus::Killed => stats.killed += 1,
            MutantStatus::Survived => stats.survived += 1,
            MutantStatus::Timeout => stats.timeout += 1,
            MutantStatus::NotCovered => stats.not_covered += 1,
            MutantStatus::CompileError => stats.compile_errors += 1,
            MutantStatus::Equivalent => {}
        }
    }

    stats
}

// ---------------------------------------------------------------------------
// Trait extension for child process wait_timeout
// ---------------------------------------------------------------------------

/// Extension trait to add `wait_timeout` to `std::process::Child`.
trait ChildExt {
    fn wait_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl ChildExt for std::process::Child {
    fn wait_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        // Poll-based wait with timeout
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Schemata;

    fn mk_mutant(id: &str, file: &str) -> Mutant {
        Mutant::new(id, file, 10, 1, "AOR", "+", "-", "test")
    }

    #[test]
    fn test_group_mutants_by_file() {
        let mutants = vec![
            mk_mutant("0", "lib/a.dart"),
            mk_mutant("1", "lib/b.dart"),
            mk_mutant("2", "lib/a.dart"),
        ];
        let groups = group_mutants_by_file(&mutants);
        assert_eq!(groups.len(), 2); // two files
    }

    #[test]
    fn test_compute_stats() {
        let results = vec![
            MutantResult::new(mk_mutant("0", "f.dart"), MutantStatus::Killed),
            MutantResult::new(mk_mutant("1", "f.dart"), MutantStatus::Survived),
            MutantResult::new(mk_mutant("2", "f.dart"), MutantStatus::Timeout),
            MutantResult::new(mk_mutant("3", "f.dart"), MutantStatus::NotCovered),
            MutantResult::new(mk_mutant("4", "f.dart"), MutantStatus::CompileError),
        ];
        let stats = compute_stats(&results, 5000);
        assert_eq!(stats.total, 5);
        assert_eq!(stats.killed, 1);
        assert_eq!(stats.survived, 1);
        assert_eq!(stats.timeout, 1);
        assert_eq!(stats.not_covered, 1);
        assert_eq!(stats.compile_errors, 1);
        assert_eq!(stats.total_duration_ms, 5000);
    }

    #[test]
    fn test_not_covered_mutant() {
        let mutant = mk_mutant("0", "lib/x.dart");
        let coverage_map = CoverageMap::default(); // empty → no covering tests
        let schemata = Schemata::new();
        let config = RunnerConfig::default();
        let timeout_calc = TimeoutCalculator::new(1000, 3.0, 5000, 300_000);

        let result = run_single_mutant(
            &mutant,
            &schemata,
            &coverage_map,
            timeout_calc.timeout_ms(),
            None,
            &config,
        );
        assert_eq!(result.status, MutantStatus::NotCovered);
    }

    #[test]
    fn test_mutant_not_in_schemata() {
        let mutant = mk_mutant("0", "lib/x.dart");
        let mut coverage_map = CoverageMap::default();
        coverage_map
            .map
            .insert("0".to_string(), vec!["test1".to_string()]);
        let schemata = Schemata::new(); // empty — mutant not in schemata
        let config = RunnerConfig::default();
        let timeout_calc = TimeoutCalculator::new(1000, 3.0, 5000, 300_000);

        let result = run_single_mutant(
            &mutant,
            &schemata,
            &coverage_map,
            timeout_calc.timeout_ms(),
            None,
            &config,
        );
        assert_eq!(result.status, MutantStatus::CompileError);
        assert!(result.message.is_some());
    }
}
