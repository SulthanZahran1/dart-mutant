//! Loop increment/decrement mutation: INC.
//!
//! Swaps `++` with `--` and vice versa. This tests whether the test suite
//! catches off-by-one errors in loop counters.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// INC — Increment/Decrement swap.
///
/// Replaces `i++` with `i--` and `i--` with `i++`.
pub struct IncrementDecrement;

impl Mutator for IncrementDecrement {
    fn name(&self) -> &str {
        "Increment/Decrement"
    }

    fn code(&self) -> &str {
        "INC"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Scan for ++ and --
            let chars: Vec<char> = clean.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let col = i + 1;
                if is_in_string_or_comment(line, col) {
                    i += 1;
                    continue;
                }

                if i + 1 < chars.len() {
                    let two = format!("{}{}", chars[i], chars[i + 1]);
                    if two == "++" {
                        // Make sure it's a post/pre increment (preceded by identifier or .)
                        // and not part of `+ +` (already handled by two-char check)
                        let before_ok = i > 0
                            && (chars[i - 1].is_alphanumeric()
                                || chars[i - 1] == '_'
                                || chars[i - 1] == '.');
                        let after_ok = i + 2 < chars.len()
                            && (chars[i + 2].is_alphanumeric() || chars[i + 2] == '_');
                        // Pre-increment: ++i (nothing before, identifier after)
                        // Post-increment: i++ (identifier before, nothing special after)
                        if before_ok || after_ok {
                            mutants.push(Mutant::without_id(
                                file_path,
                                line_no,
                                col,
                                "INC",
                                "++",
                                "--",
                                format!("INC: ++ → -- at line {}", line_no),
                            ));
                        }
                        i += 2;
                        continue;
                    }
                    if two == "--" {
                        let before_ok = i > 0
                            && (chars[i - 1].is_alphanumeric()
                                || chars[i - 1] == '_'
                                || chars[i - 1] == '.');
                        let after_ok = i + 2 < chars.len()
                            && (chars[i + 2].is_alphanumeric() || chars[i + 2] == '_');
                        if before_ok || after_ok {
                            mutants.push(Mutant::without_id(
                                file_path,
                                line_no,
                                col,
                                "INC",
                                "--",
                                "++",
                                format!("INC: -- → ++ at line {}", line_no),
                            ));
                        }
                        i += 2;
                        continue;
                    }
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
    fn test_inc_postfix() {
        let src = "for (int i = 0; i < 10; i++) {\n  print(i);\n}\n";
        let m = IncrementDecrement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "++");
        assert_eq!(mutants[0].replacement, "--");
    }

    #[test]
    fn test_inc_prefix() {
        let src = "var x = ++i;\n";
        let m = IncrementDecrement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "++");
        assert_eq!(mutants[0].replacement, "--");
    }

    #[test]
    fn test_inc_decrement() {
        let src = "i--;\n";
        let m = IncrementDecrement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "--");
        assert_eq!(mutants[0].replacement, "++");
    }

    #[test]
    fn test_inc_skips_strings() {
        let src = "var s = \"i++\";\n";
        let m = IncrementDecrement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }
}
