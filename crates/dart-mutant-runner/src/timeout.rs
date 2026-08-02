//! Adaptive per-mutant timeout calculator.
//!
//! The timeout for each mutant is based on the baseline test duration
//! multiplied by a configurable coefficient:
//!
//! ```text
//! timeout = baseline_duration_ms × coefficient
//! ```
//!
//! This is clamped to `[min_timeout_ms, max_timeout_ms]` to avoid
//! unreasonably short or long timeouts.
//!
//! The coefficient defaults to 3.0 (mutations may cause infinite loops or
//! hangs, so we give generous headroom). The user can override it with
//! `--timeout-coefficient`.

use std::time::Duration;

// ---------------------------------------------------------------------------
// TimeoutCalculator
// ---------------------------------------------------------------------------

/// Calculates adaptive per-mutant timeouts from the baseline test duration.
///
/// The formula is:
///
/// ```text
/// timeout_ms = clamp(baseline_ms × coefficient, min_ms, max_ms)
/// ```
///
/// where:
/// - `baseline_ms` is the wall-clock time of the full test suite (collected
///   during the coverage run)
/// - `coefficient` is the user-configurable multiplier (default 3.0)
/// - `min_ms` is the floor (default 5 seconds)
/// - `max_ms` is the ceiling (default 5 minutes)
#[derive(Debug, Clone)]
pub struct TimeoutCalculator {
    baseline_ms: u64,
    coefficient: f64,
    min_ms: u64,
    max_ms: u64,
    timeout_ms: u64,
}

impl TimeoutCalculator {
    /// Create a new timeout calculator.
    pub fn new(baseline_ms: u64, coefficient: f64, min_ms: u64, max_ms: u64) -> Self {
        let timeout_ms = compute_timeout(baseline_ms, coefficient, min_ms, max_ms);
        TimeoutCalculator {
            baseline_ms,
            coefficient,
            min_ms,
            max_ms,
            timeout_ms,
        }
    }

    /// The computed per-mutant timeout in milliseconds.
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// The computed per-mutant timeout as a [`Duration`].
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// The baseline duration in milliseconds.
    pub fn baseline_ms(&self) -> u64 {
        self.baseline_ms
    }

    /// The timeout coefficient.
    pub fn coefficient(&self) -> f64 {
        self.coefficient
    }

    /// The minimum timeout in milliseconds.
    pub fn min_ms(&self) -> u64 {
        self.min_ms
    }

    /// The maximum timeout in milliseconds.
    pub fn max_ms(&self) -> u64 {
        self.max_ms
    }

    /// Recalculate the timeout with a different baseline (e.g. after a
    /// faster subset run). Returns a new calculator.
    pub fn with_baseline(&self, baseline_ms: u64) -> Self {
        TimeoutCalculator::new(baseline_ms, self.coefficient, self.min_ms, self.max_ms)
    }
}

/// Compute the clamped timeout.
fn compute_timeout(baseline_ms: u64, coefficient: f64, min_ms: u64, max_ms: u64) -> u64 {
    if baseline_ms == 0 {
        // No baseline data — use the minimum as default
        return min_ms;
    }

    let raw = (baseline_ms as f64) * coefficient;
    // Clamp to [min_ms, max_ms]
    let clamped = raw.max(min_ms as f64).min(max_ms as f64);
    clamped as u64
}

// ---------------------------------------------------------------------------
// AdaptiveTimeout (per-mutant adaptive timeout record)
// ---------------------------------------------------------------------------

/// A per-mutant timeout record. Used by the scheduler to track the timeout
/// for each individual mutant.
#[derive(Debug, Clone)]
pub struct AdaptiveTimeout {
    /// Mutant ID this timeout applies to.
    pub mutant_id: String,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Whether this timeout was computed from coverage-routed tests
    /// (subset of full suite → potentially faster timeout).
    pub routed: bool,
}

impl AdaptiveTimeout {
    /// Create a new adaptive timeout record.
    pub fn new(mutant_id: impl Into<String>, timeout_ms: u64, routed: bool) -> Self {
        AdaptiveTimeout {
            mutant_id: mutant_id.into(),
            timeout_ms,
            routed,
        }
    }

    /// The timeout as a [`Duration`].
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_timeout() {
        let calc = TimeoutCalculator::new(10_000, 3.0, 5_000, 300_000);
        assert_eq!(calc.timeout_ms(), 30_000);
    }

    #[test]
    fn test_min_clamp() {
        // baseline = 1s, coefficient = 2.0 → 2s, but min is 5s
        let calc = TimeoutCalculator::new(1_000, 2.0, 5_000, 300_000);
        assert_eq!(calc.timeout_ms(), 5_000);
    }

    #[test]
    fn test_max_clamp() {
        // baseline = 200s, coefficient = 3.0 → 600s, but max is 300s
        let calc = TimeoutCalculator::new(200_000, 3.0, 5_000, 300_000);
        assert_eq!(calc.timeout_ms(), 300_000);
    }

    #[test]
    fn test_zero_baseline_uses_min() {
        let calc = TimeoutCalculator::new(0, 3.0, 5_000, 300_000);
        assert_eq!(calc.timeout_ms(), 5_000);
    }

    #[test]
    fn test_with_baseline() {
        let calc = TimeoutCalculator::new(10_000, 3.0, 5_000, 300_000);
        let calc2 = calc.with_baseline(20_000);
        assert_eq!(calc2.timeout_ms(), 60_000);
        // Original unchanged
        assert_eq!(calc.timeout_ms(), 30_000);
    }

    #[test]
    fn test_timeout_duration() {
        let calc = TimeoutCalculator::new(10_000, 3.0, 5_000, 300_000);
        assert_eq!(calc.timeout_duration(), Duration::from_millis(30_000));
    }

    #[test]
    fn test_adaptive_timeout_record() {
        let t = AdaptiveTimeout::new("M001", 30_000, true);
        assert_eq!(t.mutant_id, "M001");
        assert_eq!(t.timeout_ms, 30_000);
        assert!(t.routed);
        assert_eq!(t.duration(), Duration::from_millis(30_000));
    }

    #[test]
    fn test_fractional_coefficient() {
        // baseline = 10s, coefficient = 1.5 → 15s
        let calc = TimeoutCalculator::new(10_000, 1.5, 5_000, 300_000);
        assert_eq!(calc.timeout_ms(), 15_000);
    }
}
