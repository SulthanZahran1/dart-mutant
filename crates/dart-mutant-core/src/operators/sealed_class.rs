//! Sealed class exhaustiveness: remove a `case` from `switch` on a sealed class.
//!
//! Dart 3 introduced sealed classes with exhaustive `switch` checking. Removing
//! a `case` branch from an exhaustive switch tests whether the suite catches
//! incomplete pattern matching.
//!
//! The operator scans for `case <pattern>:` or `case <pattern> =>` lines inside
//! switch statements and removes them (replaces with a comment).

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;
use regex::Regex;

/// SealedExhaustiveness — Remove a `case` branch from a `switch`.
///
/// This is a heuristic operator: it finds `case <pattern>:` or `case <pattern> =>`
/// lines and replaces them with an empty statement (comment). This effectively
/// removes the branch from the switch, breaking exhaustiveness.
pub struct SealedExhaustivenessOperator;

impl Mutator for SealedExhaustivenessOperator {
    fn name(&self) -> &str {
        "SealedExhaustiveness"
    }

    fn code(&self) -> &str {
        "SealedExhaustiveness"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Match `case <pattern>:` or `case <pattern> =>`
        // The pattern can be an identifier, a type, a constant, etc.
        let re = Regex::new(r"\bcase\s+(?P<pattern>[^:]+?)(?P<colon>:|=>)").unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Only consider lines inside a switch (heuristic: line starts with `case`)
            if !clean.trim().starts_with("case ") {
                continue;
            }

            for m in re.captures_iter(clean) {
                let pattern = m.name("pattern").map(|p| p.as_str()).unwrap_or("");
                let colon = m.name("colon").map(|c| c.as_str()).unwrap_or(":");
                if pattern.is_empty() {
                    continue;
                }

                let full_start = m.get(0).unwrap().start();
                let col = full_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                let original = format!("case {}{}", pattern, colon);
                let replacement = "/* SealedExhaustiveness: case removed */".to_string();
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "SealedExhaustiveness",
                    &original,
                    &replacement,
                    format!(
                        "SealedExhaustiveness: remove `case {}` at line {}",
                        pattern.trim(),
                        line_no
                    ),
                ));
            }
        }

        mutants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sealed_finds_case_colon() {
        let src = "switch (shape) {\n  case Circle:\n    return 'circle';\n  case Square:\n    return 'square';\n}\n";
        let m = SealedExhaustivenessOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 2);
        assert!(mutants.iter().all(|m| m.operator == "SealedExhaustiveness"));
    }

    #[test]
    fn test_sealed_finds_arrow_case() {
        let src = "switch (x) {\n  case 1 => 'one',\n  case 2 => 'two',\n}\n";
        let m = SealedExhaustivenessOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 2);
    }

    #[test]
    fn test_sealed_skips_non_case_lines() {
        let src = "var x = 1;\nreturn x;\n";
        let m = SealedExhaustivenessOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }
}
