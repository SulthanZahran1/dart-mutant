//! Kernel-bytecode comparison for Trivial Compiler Equivalence.
//!
//! Compiles original and mutated Dart source with `dart compile kernel` and
//! compares the resulting `.dill` files byte-for-byte.

use anyhow::{Context, Result};
use log::{debug, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use dart_mutant_core::{Mutant, MutantStatus};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The result of a TCE check for a single mutant.
#[derive(Debug, Clone)]
pub struct TceOutcome {
    pub status: MutantStatus,
    /// Human-readable detail for logging / reports.
    pub detail: String,
}

impl TceOutcome {
    pub fn equivalent(detail: impl Into<String>) -> Self {
        Self {
            status: MutantStatus::Equivalent,
            detail: detail.into(),
        }
    }
    pub fn not_equivalent(detail: impl Into<String>) -> Self {
        Self {
            status: MutantStatus::Survived,
            detail: detail.into(),
        }
    }
    pub fn compile_error(detail: impl Into<String>) -> Self {
        Self {
            status: MutantStatus::CompileError,
            detail: detail.into(),
        }
    }
    pub fn error(detail: impl Into<String>) -> Self {
        Self {
            status: MutantStatus::CompileError,
            detail: detail.into(),
        }
    }
}

/// Convenience: return `true` if `original_source` and `mutated_source` produce
/// identical kernel bytecode.
///
/// Returns `false` for any non-equivalent outcome (compile error, infra error,
/// or genuinely different bytecode). See [`tce_check`] for full details.
pub fn is_equivalent(original_source: &str, mutated_source: &str) -> bool {
    matches!(
        tce_check_raw(original_source, mutated_source).status,
        MutantStatus::Equivalent
    )
}

/// Full TCE check using a [`Mutant`] for metadata and explicit source strings.
///
/// The [`Mutant`] provides the id / file / line for logging. The full original
/// and mutated source are passed separately because the `Mutant` struct stores
/// only the mutation-point snippet (`original` / `replacement`), not the
/// complete file content needed for compilation.
pub fn tce_check(mutant: &Mutant, original_source: &str, mutated_source: &str) -> TceOutcome {
    match run_tce(original_source, mutated_source, Some(&mutant.id)) {
        Ok(true) => TceOutcome::equivalent(format!(
            "TCE: kernel bytecode identical for mutant {}",
            mutant.id
        )),
        Ok(false) => TceOutcome::not_equivalent(format!(
            "TCE: kernel bytecode differs for mutant {}",
            mutant.id
        )),
        Err(e) => {
            let msg = format!("TCE error for mutant {}: {e:#}", mutant.id);
            warn!("{msg}");
            TceOutcome::error(msg)
        }
    }
}

/// Full TCE check using only source strings (no [`Mutant`] metadata).
///
/// Useful when you have raw source and don't need mutant-id logging.
pub fn tce_check_raw(original_source: &str, mutated_source: &str) -> TceOutcome {
    match run_tce(original_source, mutated_source, None) {
        Ok(true) => TceOutcome::equivalent("TCE: kernel bytecode identical"),
        Ok(false) => TceOutcome::not_equivalent("TCE: kernel bytecode differs"),
        Err(e) => {
            let msg = format!("TCE error: {e:#}");
            warn!("{msg}");
            TceOutcome::error(msg)
        }
    }
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/// Inner implementation returning `Ok(bool)` where the bool means "equivalent".
fn run_tce(original_source: &str, mutated_source: &str, mutant_id: Option<&str>) -> Result<bool> {
    let tmp = TempDir::new().context("failed to create temp directory for TCE")?;

    // IMPORTANT: `dart compile kernel` embeds the SOURCE FILE PATH in the
    // .dill bytecode. Compiling the same source from two different filenames
    // (e.g. original.dart vs mutant.dart) yields different kernels, which
    // would make every mutant look non-equivalent. We therefore reuse the
    // SAME source path for both compilations — write original, compile, then
    // overwrite the same file with the mutant and compile again.
    let source_dart = tmp.path().join("source.dart");
    let original_dill = tmp.path().join("original.dill");
    let mutant_dill = tmp.path().join("mutant.dill");

    // Write + compile original
    fs::write(&source_dart, original_source).with_context(|| {
        format!(
            "failed to write original source to {}",
            source_dart.display()
        )
    })?;
    compile_kernel(&source_dart, &original_dill)
        .context("compiling original source with `dart compile kernel`")?;

    // Overwrite the same path with the mutant, compile again
    fs::write(&source_dart, mutated_source).with_context(|| {
        format!(
            "failed to write mutated source to {}",
            source_dart.display()
        )
    })?;
    compile_kernel(&source_dart, &mutant_dill).with_context(|| {
        format!(
            "compiling mutated source with `dart compile kernel`{}",
            mutant_id
                .map(|id| format!(" (mutant {id})"))
                .unwrap_or_default()
        )
    })?;

    // Compare byte-for-byte
    let equivalent = compare_files(&original_dill, &mutant_dill)?;
    debug!(
        "TCE {}: {} — original {} bytes, mutant {} bytes, equivalent={}",
        mutant_id.unwrap_or("-"),
        if equivalent { "EQUIVALENT" } else { "DIFFERS" },
        fs::metadata(&original_dill).map(|m| m.len()).unwrap_or(0),
        fs::metadata(&mutant_dill).map(|m| m.len()).unwrap_or(0),
        equivalent,
    );
    Ok(equivalent)
}

/// Run `dart compile kernel <source> -o <output>` and check it succeeded.
fn compile_kernel(source: &Path, output: &Path) -> Result<()> {
    debug!(
        "dart compile kernel {} -o {}",
        source.display(),
        output.display()
    );
    let result = Command::new("dart")
        .args(["compile", "kernel"])
        .arg(source)
        .arg("-o")
        .arg(output)
        .output()
        .context("failed to spawn `dart compile kernel` — is Dart SDK on PATH?")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        anyhow::bail!(
            "dart compile kernel failed (exit {})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            result.status
        );
    }
    Ok(())
}

/// Compare two files byte-for-byte. Returns `Ok(true)` if identical.
fn compare_files(a: &Path, b: &Path) -> Result<bool> {
    let a_bytes = fs::read(a).with_context(|| format!("reading {}", a.display()))?;
    let b_bytes = fs::read(b).with_context(|| format!("reading {}", b.display()))?;
    Ok(a_bytes == b_bytes)
}

/// Build the expected `.dill` output path for a given source path.
///
/// `dart compile kernel foo.dart` produces `foo.dill` next to the source
/// unless `-o` is specified. This helper is exposed for callers that don't
/// use `-o` (though the internal implementation always does).
#[allow(dead_code)]
fn default_dill_path(source: &Path) -> PathBuf {
    let stem = source.file_stem().unwrap_or_default();
    let mut out = source.to_path_buf();
    out.set_file_name(stem);
    out.set_extension("dill");
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use dart_mutant_core::Mutant;

    fn make_mutant() -> Mutant {
        Mutant {
            id: "test:1:TEST".into(),
            file_path: "test.dart".into(),
            line: 1,
            column: 1,
            operator: "TEST".into(),
            original: "+".into(),
            replacement: "-".into(),
            description: "test mutant".into(),
        }
    }

    fn dart_available() -> bool {
        Command::new("dart")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn identical_source_is_equivalent() {
        // When original and mutated source are identical, the bytecode must be
        // identical → Equivalent. This is the baseline / sanity check.
        if !dart_available() {
            eprintln!("skipping identical_source_is_equivalent: dart not on PATH");
            return;
        }
        let source = "int add(int a, int b) => a + b;\nvoid main() { print(add(1, 2)); }\n";
        let outcome = tce_check_raw(source, source);
        assert_eq!(
            outcome.status,
            MutantStatus::Equivalent,
            "identical source should be TCE-equivalent, got: {}",
            outcome.detail
        );
    }

    #[test]
    fn different_source_is_not_equivalent() {
        if !dart_available() {
            eprintln!("skipping different_source_is_not_equivalent: dart not on PATH");
            return;
        }
        let original = "int add(int a, int b) => a + b;\nvoid main() { print(add(1, 2)); }\n";
        let mutated = "int add(int a, int b) => a - b;\nvoid main() { print(add(1, 2)); }\n";
        let outcome = tce_check_raw(original, mutated);
        assert_ne!(
            outcome.status,
            MutantStatus::Equivalent,
            "a + b → a - b should NOT be equivalent, got: {}",
            outcome.detail
        );
    }

    #[test]
    fn invalid_mutant_is_compile_error() {
        if !dart_available() {
            eprintln!("skipping invalid_mutant_is_compile_error: dart not on PATH");
            return;
        }
        let original = "int add(int a, int b) => a + b;\nvoid main() { print(add(1, 2)); }\n";
        let mutated = "int add(int a, int b) => a + ;\nvoid main() { print(add(1, 2)); }\n";
        let outcome = tce_check_raw(original, mutated);
        assert_eq!(
            outcome.status,
            MutantStatus::CompileError,
            "syntactically invalid mutant should be CompileError, got: {}",
            outcome.detail
        );
    }

    #[test]
    fn missing_dart_is_handled_gracefully() {
        // This test only runs when dart is NOT available.
        if dart_available() {
            eprintln!("skipping missing_dart_is_handled_gracefully: dart IS on PATH");
            return;
        }
        let source = "void main() {}\n";
        let outcome = tce_check_raw(source, source);
        assert_eq!(
            outcome.status,
            MutantStatus::CompileError,
            "missing dart should surface as CompileError, got: {}",
            outcome.detail
        );
    }

    #[test]
    fn default_dill_path_works() {
        let p = default_dill_path(Path::new("/tmp/foo.dart"));
        assert_eq!(p, PathBuf::from("/tmp/foo.dill"));
    }

    #[test]
    fn is_equivalent_wrapper_no_panic() {
        let source = "void main() {}\n";
        // Should not panic regardless of dart availability
        let _ = is_equivalent(source, source);
    }

    #[test]
    fn tce_check_with_mutant_metadata() {
        let source = "void main() {}\n";
        let mutant = make_mutant();
        // Should not panic regardless of dart availability
        let outcome = tce_check(&mutant, source, source);
        // If dart is available, identical source → Equivalent
        if dart_available() {
            assert_eq!(outcome.status, MutantStatus::Equivalent);
        }
    }
}
