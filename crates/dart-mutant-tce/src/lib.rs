//! Trivial Compiler Equivalence (TCE) detection for dart_mutant.
//!
//! TCE identifies "equivalent mutants" — mutations that produce semantically
//! identical code and can never be killed by any test. The approach:
//!
//! 1. Write the original source to a temp file and compile it with
//!    `dart compile kernel` → produces a `.dill` (kernel bytecode) file.
//! 2. Write the mutated source to a temp file and compile it the same way.
//! 3. Compare the two `.dill` files byte-for-byte.
//! 4. If identical → the mutant is `MutantStatus::Equivalent`.
//!
//! This is a conservative technique: a byte-identical kernel guarantees the
//! mutation had no observable effect on the compiled program, so no test
//! can distinguish the mutant from the original. False negatives are possible
//! (a mutant might be equivalent but produce different bytecode due to
//! debug info / source-map differences), but false positives are not —
//! identical bytecode means equivalent behavior.

pub mod kernel_compare;

pub use kernel_compare::{is_equivalent, tce_check, tce_check_raw, TceOutcome};
