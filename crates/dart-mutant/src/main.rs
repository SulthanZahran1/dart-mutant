//! dart_mutant CLI — entry point and pipeline orchestration.
//!
//! Pipeline stages (in order):
//!   1. Parse CLI args → merge with `.dart_mutant.yml` + defaults → [`Config`]
//!   2. Scan source files and discover mutation points via operators
//!   3. Collect coverage via dart_mutant_runner
//!   4. Build per-test coverage map via dart_mutant_runner
//!   5. Build mutant schemata (compile once)
//!   6. Run mutants in parallel via dart_mutant_runner
//!   7. TCE equivalent detection via dart_mutant_tce (optional)
//!   8. Generate reports via dart_mutant_report
//!   9. Print console summary
//!  10. Exit with correct code (0=pass, 1=below threshold, 2=error)

mod cli;
mod config;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use cli::Cli;
use config::Config;

use dart_mutant_core::{
    all_operators, mutation_coverage, mutation_score, Mutant, MutantResult, MutantStatus,
};
use dart_mutant_report as report_lib;
use dart_mutant_runner as runner_lib;
use dart_mutant_tce as tce_lib;

/// Exit code: MSI ≥ threshold (pass).
const EXIT_PASS: u8 = 0;
/// Exit code: MSI < threshold (below threshold).
const EXIT_BELOW_THRESHOLD: u8 = 1;
/// Exit code: error occurred.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// Top-level orchestration: parse args, run pipeline, return exit code.
fn run() -> Result<u8> {
    let cli = Cli::parse();

    // Logging is normally suppressed by `--quiet` (stdout must stay pure
    // JSON for `--format json`). DM_FORCE_LOG=1 overrides that so CI can
    // capture the tool's internal diagnostics even in quiet mode.
    if !cli.quiet || std::env::var("DM_FORCE_LOG").is_ok() {
        let _ = env_logger::Builder::from_env(env_logger::Env::default())
            .format_timestamp(None)
            .try_init();
    }

    let config = Config::from_cli(&cli)?;
    let summary = pipeline(&config)?;

    let exit_code = if summary.mutation_score >= config.threshold {
        EXIT_PASS
    } else {
        EXIT_BELOW_THRESHOLD
    };

    Ok(exit_code)
}

/// Mutation testing result summary — used for JSON output and console display.
///
/// `schema_version` is declared first so serde serializes it as the opening
/// key of the JSON object (serde emits struct fields in declaration order).
/// It is the frozen agent-facing JSON contract — see
/// `docs/cli-json-contract.md`. Bump only on a breaking change (major
/// version, e.g. `2.0`); additive changes keep `1.0`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSummary {
    /// Agent-facing JSON schema version. Serialized as `schemaVersion`.
    /// Frozen at `"1.0"` — additive-only changes keep this value; breaking
    /// changes require a major bump (e.g. `2.0`).
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub mutation_score: f64,
    pub mutation_coverage: f64,
    pub total: usize,
    pub killed: usize,
    pub survived: usize,
    pub timeout: usize,
    pub equivalent: usize,
    pub not_covered: usize,
    pub compile_error: usize,
    pub threshold: f64,
    pub passed: bool,
    pub files: Vec<FileSummary>,
}

/// Per-file mutation summary.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSummary {
    pub path: String,
    pub mutation_score: f64,
    pub killed: usize,
    pub survived: usize,
    pub total: usize,
}

impl Default for PipelineSummary {
    fn default() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            mutation_score: 0.0,
            mutation_coverage: 0.0,
            total: 0,
            killed: 0,
            survived: 0,
            timeout: 0,
            equivalent: 0,
            not_covered: 0,
            compile_error: 0,
            threshold: 0.0,
            passed: false,
            files: Vec::new(),
        }
    }
}

/// Scan Dart source files under `source_path`, excluding `exclude` patterns.
fn scan_source_files(
    project_path: &Path,
    source_path: &str,
    exclude: &[String],
) -> Vec<(PathBuf, String)> {
    let full_source = project_path.join(source_path);
    let mut files = Vec::new();

    if !full_source.is_dir() {
        return files;
    }

    for entry in walkdir::WalkDir::new(&full_source)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("dart") {
            continue;
        }

        let rel = path
            .strip_prefix(project_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Check exclusions
        let excluded = exclude.iter().any(|pattern| {
            rel.contains(pattern.trim_end_matches("**"))
                || rel.ends_with(pattern)
                || glob_match(pattern, &rel)
        });
        if excluded {
            continue;
        }

        if let Ok(source) = std::fs::read_to_string(path) {
            // Store the RELATIVE path (project-root-relative) as the file
            // identity, with FORWARD slashes on every platform. Coverage JSON
            // sources are `file:///...` URIs (always forward-slash), so suffix
            // matching against `lib/x.dart` works on Linux AND Windows. The
            // canonical absolute form breaks this on Windows (verbatim
            // `\\?\D:\...` prefix) and so does `lib\x.dart` (backslashes).
            files.push((PathBuf::from(rel.replace('\\', "/")), source));
        }
    }

    files
}

/// Simple glob matcher for exclude patterns.
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            return text.starts_with(parts[0].trim_start_matches('/'))
                && text.ends_with(parts[1].trim_start_matches('/'));
        }
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return text.starts_with(parts[0]) && text.ends_with(parts[1]);
        }
    }
    text == pattern
}

/// Discover all mutation points across source files using registered operators.
fn discover_mutations(
    source_files: &[(PathBuf, String)],
    operator_filter: &[String],
) -> Vec<Mutant> {
    let operators = all_operators();
    let mut mutants = Vec::new();
    let mut id_counter = 0u64;

    for (path, source) in source_files {
        let file_path = path.to_string_lossy().to_string();

        for op in &operators {
            // Filter by operator code if a filter is set
            if !operator_filter.is_empty()
                && !operator_filter
                    .iter()
                    .any(|f| f == op.code() || f == op.name())
            {
                continue;
            }

            let found = op.find_mutations(source, &file_path);
            for mut m in found {
                m.id = format!("{}", id_counter);
                id_counter += 1;
                mutants.push(m);
            }
        }
    }

    mutants
}

/// The full mutation testing pipeline.
fn pipeline(config: &Config) -> Result<PipelineSummary> {
    let json_only = config.suppress_progress();

    // Stage 1: Scan source files
    if !json_only {
        eprint!("Parsing source files… ");
    }
    let source_files = scan_source_files(&config.path, &config.source_path, &config.exclude);
    if !json_only {
        eprintln!("{} files", source_files.len());
    }

    if source_files.is_empty() {
        anyhow::bail!(
            "no Dart source files found in {}",
            config.path.join(&config.source_path).display()
        );
    }

    // Stage 2: Discover mutation points
    if !json_only {
        eprint!("Discovering mutation points… ");
    }
    let mut mutants = discover_mutations(&source_files, &config.operators);

    // Apply --sample limit
    if let Some(n) = config.sample {
        if mutants.len() > n {
            mutants.truncate(n);
        }
    }

    // Apply --mutant filter
    if let Some(id) = config.mutant_id {
        let target = format!("{}", id);
        mutants.retain(|m| m.id == target);
    }

    if !json_only {
        eprintln!("{} mutants found", mutants.len());
    }

    if mutants.is_empty() {
        if !json_only {
            eprintln!("No mutants to test — nothing to do.");
        }
        return Ok(PipelineSummary {
            threshold: config.threshold,
            ..Default::default()
        });
    }

    // Stage 3: Build schemata from source files
    if !json_only {
        eprint!("Building schemata… ");
    }
    let sources_map: HashMap<String, String> = source_files
        .iter()
        .map(|(p, s)| (p.to_string_lossy().to_string(), s.clone()))
        .collect();
    let schemata = runner_lib::Schemata::from_mutants(&mutants, &sources_map);
    if !json_only {
        eprintln!("done");
    }

    // Stage 4: Run the mutation testing pipeline via the runner
    let runner_config =
        runner_lib::RunnerConfig::new(config.path.clone(), config.test_command.clone())
            .with_parallel(config.parallel)
            .with_timeout_coefficient(config.timeout_coefficient);

    let results = runner_lib::run(&mutants, &schemata, &runner_config)?;

    // Stage 5: TCE equivalent detection (optional)
    let mut results = results;
    if config.detect_equivalent {
        if !json_only {
            eprint!("Detecting equivalent mutants (TCE)… ");
        }
        for result in &mut results {
            if result.status == MutantStatus::Survived {
                let file_path = config.path.join(&result.mutant.file_path);
                if let Ok(original_source) = std::fs::read_to_string(&file_path) {
                    // Generate mutated source
                    let mutated_source = apply_mutation_to_source(&original_source, &result.mutant);
                    if tce_lib::is_equivalent(&original_source, &mutated_source) {
                        result.status = MutantStatus::Equivalent;
                    }
                }
            }
        }
        if !json_only {
            let eq_count = results
                .iter()
                .filter(|r| r.status == MutantStatus::Equivalent)
                .count();
            eprintln!("{} equivalent mutants detected", eq_count);
        }
    }

    // Stage 6: Build summary
    let summary = build_summary(&results, config);

    // Stage 7: Generate reports
    // JSON output goes to stdout
    if config.wants_json() {
        let json = serde_json::to_string_pretty(&summary)?;
        println!("{json}");
    }

    // HTML report
    if config.wants_html() {
        let report_dir = config.path.join("mutation-reports");
        std::fs::create_dir_all(&report_dir)?;
        let html_path = report_dir.join("mutation-report.html");
        let html = report_lib::html::generate(&results)?;
        report_lib::write_report_to_file(&html_path, &html)?;
        if !json_only {
            eprintln!("  → {}", html_path.display());
        }
    }

    // JUnit XML report
    if config.wants_junit() {
        let report_dir = config.path.join("mutation-reports");
        std::fs::create_dir_all(&report_dir)?;
        let junit_path = report_dir.join("mutation-results.xml");
        let junit = report_lib::junit_xml::generate(&results)?;
        report_lib::write_report_to_file(&junit_path, &junit)?;
        if !json_only {
            eprintln!("  → {}", junit_path.display());
        }
    }

    // Stryker JSON report
    if config.wants_json() {
        let report_dir = config.path.join("mutation-reports");
        std::fs::create_dir_all(&report_dir)?;
        let stryker_path = report_dir.join("mutation-report.json");
        let stryker = report_lib::stryker_json::generate(&results)?;
        report_lib::write_report_to_file(&stryker_path, &stryker)?;
    }

    // Console summary
    if config.wants_console() && !json_only {
        print_console_summary(&summary);
    }

    Ok(summary)
}

/// Apply a mutation to source code for TCE comparison.
fn apply_mutation_to_source(source: &str, mutant: &Mutant) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    if mutant.line == 0 || mutant.line > result.len() {
        return source.to_string();
    }

    let idx = mutant.line - 1;
    let line = &result[idx];

    if mutant.original.is_empty() {
        result[idx] = String::new();
    } else if line.contains(&mutant.original) {
        result[idx] = line.replacen(&mutant.original, &mutant.replacement, 1);
    }

    result.join("\n")
}

/// Build the [`PipelineSummary`] from raw mutant results.
fn build_summary(results: &[MutantResult], config: &Config) -> PipelineSummary {
    let total = results.len();
    let killed = results
        .iter()
        .filter(|r| r.status == MutantStatus::Killed)
        .count();
    let survived = results
        .iter()
        .filter(|r| r.status == MutantStatus::Survived)
        .count();
    let timeout = results
        .iter()
        .filter(|r| r.status == MutantStatus::Timeout)
        .count();
    let equivalent = results
        .iter()
        .filter(|r| r.status == MutantStatus::Equivalent)
        .count();
    let not_covered = results
        .iter()
        .filter(|r| r.status == MutantStatus::NotCovered)
        .count();
    let compile_error = results
        .iter()
        .filter(|r| r.status == MutantStatus::CompileError)
        .count();

    let ms = mutation_score(results);
    let mc = mutation_coverage(results);
    let passed = ms >= config.threshold;

    // Per-file breakdown
    let mut files_map: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for r in results {
        let entry = files_map
            .entry(r.mutant.file_path.clone())
            .or_insert((0, 0, 0));
        entry.2 += 1;
        if r.status == MutantStatus::Killed {
            entry.0 += 1;
        } else if r.status == MutantStatus::Survived {
            entry.1 += 1;
        }
    }
    let files: Vec<FileSummary> = files_map
        .into_iter()
        .map(|(path, (k, s, t))| FileSummary {
            path,
            mutation_score: if t > 0 {
                k as f64 / t as f64 * 100.0
            } else {
                0.0
            },
            killed: k,
            survived: s,
            total: t,
        })
        .collect();

    PipelineSummary {
        schema_version: "1.0".to_string(),
        mutation_score: ms,
        mutation_coverage: mc,
        total,
        killed,
        survived,
        timeout,
        equivalent,
        not_covered,
        compile_error,
        threshold: config.threshold,
        passed,
        files,
    }
}

/// Print the human-readable console summary to stderr.
fn print_console_summary(summary: &PipelineSummary) {
    eprintln!();
    eprintln!("Results:");
    eprintln!(
        "  Killed:         {:>5}  ({:.1}%)",
        summary.killed,
        pct(summary.killed, summary.total)
    );
    eprintln!(
        "  Survived:       {:>5}  ({:.1}%)",
        summary.survived,
        pct(summary.survived, summary.total)
    );
    eprintln!(
        "  Timeout:        {:>5}  ({:.1}%)",
        summary.timeout,
        pct(summary.timeout, summary.total)
    );
    eprintln!("  Equivalent:     {:>5}  (excluded)", summary.equivalent);
    eprintln!("  Not covered:    {:>5}  (excluded)", summary.not_covered);
    eprintln!(
        "  Compile error:  {:>5}  ({:.1}%)",
        summary.compile_error,
        pct(summary.compile_error, summary.total)
    );
    eprintln!();
    eprintln!("  Mutation Score (MSI): {:.1}%", summary.mutation_score);
    eprintln!("  Mutation Coverage:   {:.1}%", summary.mutation_coverage);
    eprintln!(
        "  Threshold: {}%       {}",
        summary.threshold,
        pass_fail_mark(summary.passed)
    );
    eprintln!();
}

/// Percentage helper: `n / total × 100`.
fn pct(n: usize, total: usize) -> f64 {
    if total > 0 {
        n as f64 / total as f64 * 100.0
    } else {
        0.0
    }
}

/// Pass/fail marker for console output.
fn pass_fail_mark(passed: bool) -> &'static str {
    if passed {
        "✅ PASSED"
    } else {
        "❌ FAILED"
    }
}
