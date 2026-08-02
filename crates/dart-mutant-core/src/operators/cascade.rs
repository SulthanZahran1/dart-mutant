//! Cascade operator: `..` → `.`.
//!
//! Dart's cascade operator (`..`) allows chaining method calls on the same
//! object without losing the reference. Converting `..` to `.` changes the
//! return value — the cascade returns the original object, while `.` returns
//! the method's return value.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// Cascade — Replace `..` with `.` (cascade to dot).
///
/// `a..b()` → `a.b()` (changes the return value from `a` to the return of `b()`).
pub struct CascadeOperator;

impl Mutator for CascadeOperator {
    fn name(&self) -> &str {
        "Cascade"
    }

    fn code(&self) -> &str {
        "Cascade"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            let chars: Vec<char> = clean.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let col = i + 1;
                if is_in_string_or_comment(line, col) {
                    i += 1;
                    continue;
                }

                // Look for `..` (cascade operator)
                if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1] == '.' {
                    // Make sure it's not `...` (spread operator)
                    if i + 2 < chars.len() && chars[i + 2] == '.' {
                        // This is `...` (spread) — skip
                        i += 3;
                        continue;
                    }

                    // Cascade `..` can be preceded by an identifier, `)`, `]`, `}`,
                    // or whitespace (when the cascade is on a new line after the
                    // receiver on the previous line).
                    if i > 0 {
                        let before = chars[i - 1];
                        if before.is_whitespace() {
                            // Look further back for a non-whitespace char
                            // (the receiver is on a previous line or earlier)
                            // This is valid cascade syntax — proceed.
                        } else if !(before.is_alphanumeric()
                            || before == '_'
                            || before == ')'
                            || before == ']'
                            || before == '}')
                        {
                            i += 2;
                            continue;
                        }
                    }
                    // Must be followed by something (method/property name)
                    if i + 2 >= chars.len() {
                        i += 2;
                        continue;
                    }
                    let after = chars[i + 2];
                    if !(after.is_alphanumeric() || after == '_') {
                        i += 2;
                        continue;
                    }

                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "Cascade",
                        "..",
                        ".",
                        format!("Cascade: .. → . at line {}", line_no),
                    ));
                    i += 2;
                    continue;
                }

                i += 1;
            }
        }

        mutants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cascade_finds_mutations() {
        let src = "var obj = MyClass()\n  ..field = 1\n  ..method();\n";
        let m = CascadeOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 2); // Two `..` operators
        assert!(mutants.iter().all(|m| m.operator == "Cascade"));
    }

    #[test]
    fn test_cascade_skips_spread() {
        let src = "var list = [...other, 1, 2];\n";
        let m = CascadeOperator;
        let mutants = m.find_mutations(src, "test.dart");
        // `...` should not be mutated
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_cascade_skips_strings() {
        let src = "var s = \"a..b\";\n";
        let m = CascadeOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }
}
