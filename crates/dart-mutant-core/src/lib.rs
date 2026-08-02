//! dart-mutant-core: Core mutation engine for dart_mutant.
//!
//! This crate contains:
//! - The [`Mutant`], [`MutantId`], [`MutationPoint`], and [`MutantStatus`] types
//! - The [`Mutator`] trait and all built-in mutation operators
//! - The [schemata generator](schemata) for compile-once mutation testing
//!
//! ## Overview
//!
//! Mutation testing injects small, deliberate faults (mutants) into source
//! code and runs the test suite against each one. If the tests still pass,
//! the mutant "survived" — revealing a gap in test coverage.
//!
//! dart-mutant-core implements line-based source mutation (find/replace byte
//! ranges on source text) using regex and line scanning. Each operator
//! implements the [`Mutator`] trait, which finds mutation points in source
//! text and produces [`Mutant`] structs with replacement text.
//!
//! ## Operators
//!
//! ### Generic operators
//! - AOR — Arithmetic Operator Replacement (`+` → `-`, `*` → `/`, etc.)
//! - AOD — Arithmetic Operator Deletion (`a + b` → `a`)
//! - AOI — Arithmetic Operator Insertion (`a` → `a + 1`)
//! - ROR — Relational Operator Replacement (`>` → `>=`, `==` → `!=`, etc.)
//! - LOR — Logical Operator Replacement (`&&` → `||`)
//! - LCR — Logical Constant Replacement (`true` → `false`)
//! - COR — Conditional Operator Replacement (`if (x)` → `if (!x)`)
//! - SDL — Statement Deletion (remove a statement)
//! - RVR — Return Value Replacement (replace return with zero/empty)
//! - INC — Increment/Decrement swap (`i++` → `i--`)
//!
//! ### Dart-specific operators
//! - NullSafety — `a ?? b` → `a` (remove null fallback)
//! - NullAssert — `a!` → `a` (remove null assertion)
//! - OptionalChaining — `a?.b` → `a.b` (remove safe member access)
//! - Cascade — `a..b()` → `a.b()` (cascade to dot)
//! - AsyncAwait — `await f()` → `f()` (remove await)
//! - StreamMutation — `.first` → `.last`, `.isEmpty` → `.isNotEmpty`, etc.
//! - SealedExhaustiveness — remove a `case` from `switch` on sealed class

pub mod mutant;
pub mod operators;
pub mod schemata;

// Re-export the main types at the crate root for convenience.
pub use mutant::{
    mutation_coverage, mutation_score, Mutant, MutantId, MutantResult, MutantStatus, MutationPoint,
};
pub use operators::{all_operators, dart_specific_codes, generic_codes, Mutator};
pub use schemata::{generate_project_schemata, generate_schemata, MUTANT_ENV_VAR};

/// Crate version (matches Cargo.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_all_operators_find_mutations() {
        let src = r#"
int add(int a, int b) => a + b;
bool isGreater(int a, int b) => a > b;
bool check(int x) {
  if (x > 0 && x < 100) return true;
  return false;
}
void loop() {
  for (int i = 0; i < 10; i++) {
    print(i);
  }
}
var value = nullable ?? defaultValue;
var safe = obj?.value;
var asserted = nullable!;
var cascaded = MyClass()..field = 1..method();
Future<int> fetch() async => await loadData();
var first = stream.first;
"#;
        let file_path = "test.dart";
        let mut total_mutants = 0;
        for op in all_operators() {
            let mutants = op.find_mutations(src, file_path);
            if !mutants.is_empty() {
                println!("{}: {} mutants", op.code(), mutants.len());
            }
            total_mutants += mutants.len();
        }
        assert!(total_mutants > 0, "should find at least some mutants");
    }

    #[test]
    fn test_schemata_generation_roundtrip() {
        let src = "int add(int a, int b) => a + b;\n";
        let aor = operators::arithmetic::ArithmeticOperatorReplacement;
        let mutants = aor.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());

        // Assign ids
        let mutants: Vec<Mutant> = mutants
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let mut m = m.clone();
                m.id = i.to_string();
                m
            })
            .collect();

        let schemata = generate_schemata(src, &mutants);
        assert!(schemata.contains("String.fromEnvironment"));
        assert!(schemata.contains("DART_MUTANT_ID"));
    }

    #[test]
    fn test_mutant_status_scoring() {
        let m = Mutant::without_id("f.dart", 1, 1, "AOR", "+", "-", "test");
        let results = vec![
            MutantResult::new(m.clone(), MutantStatus::Killed),
            MutantResult::new(m.clone(), MutantStatus::Survived),
            MutantResult::new(m.clone(), MutantStatus::Timeout),
            MutantResult::new(m.clone(), MutantStatus::Equivalent),
            MutantResult::new(m, MutantStatus::NotCovered),
        ];
        let score = mutation_score(&results);
        // is_killed (Killed + Timeout) = 2, scored = 3 → 66.67%
        assert!((score - 66.666).abs() < 0.1);
    }

    #[test]
    fn test_dart_specific_operator_count() {
        let codes = dart_specific_codes();
        assert!(codes.len() >= 6, "need at least 6 Dart-specific operators");
    }

    #[test]
    fn test_generic_operator_count() {
        let codes = generic_codes();
        assert!(codes.len() >= 7, "need at least 7 generic operators");
    }
}
