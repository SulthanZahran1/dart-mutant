//! Logical operators: LOR (operator replacement) and LCR (constant replacement).
//!
//! - **LOR** — Logical Operator Replacement: `&&` → `||`, `||` → `&&`.
//! - **LCR** — Logical Constant Replacement: `true` → `false`, `false` → `true`.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// LOR — Logical Operator Replacement.
///
/// Replaces `&&` with `||` and vice versa.
pub struct LogicalOperatorReplacement;

impl Mutator for LogicalOperatorReplacement {
    fn name(&self) -> &str {
        "Logical Operator Replacement"
    }

    fn code(&self) -> &str {
        "LOR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Scan for && and ||
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
                    if two == "&&" {
                        mutants.push(Mutant::without_id(
                            file_path,
                            line_no,
                            col,
                            "LOR",
                            "&&",
                            "||",
                            format!("LOR: && → || at line {}", line_no),
                        ));
                        i += 2;
                        continue;
                    }
                    if two == "||" {
                        mutants.push(Mutant::without_id(
                            file_path,
                            line_no,
                            col,
                            "LOR",
                            "||",
                            "&&",
                            format!("LOR: || → && at line {}", line_no),
                        ));
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

/// LCR — Logical Constant Replacement.
///
/// Replaces `true` with `false` and `false` with `true`.
pub struct LogicalConstantReplacement;

impl Mutator for LogicalConstantReplacement {
    fn name(&self) -> &str {
        "Logical Constant Replacement"
    }

    fn code(&self) -> &str {
        "LCR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Find standalone `true` and `false` keywords (word-boundary match).
            scan_for_word(clean, "true", |pos, matched| {
                let col = pos + 1;
                if is_in_string_or_comment(line, col) {
                    return;
                }
                // Ensure it's a standalone word (not part of an identifier like `trueValue`)
                if !is_word_boundary(clean, pos, matched.len()) {
                    return;
                }
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "LCR",
                    "true",
                    "false",
                    format!("LCR: true → false at line {}", line_no),
                ));
            });

            scan_for_word(clean, "false", |pos, matched| {
                let col = pos + 1;
                if is_in_string_or_comment(line, col) {
                    return;
                }
                if !is_word_boundary(clean, pos, matched.len()) {
                    return;
                }
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "LCR",
                    "false",
                    "true",
                    format!("LCR: false → true at line {}", line_no),
                ));
            });
        }

        mutants
    }
}

/// Scan for all occurrences of `word` in `line` and call `f` with the byte position and the matched text.
fn scan_for_word<F>(line: &str, word: &str, mut f: F)
where
    F: FnMut(usize, &str),
{
    let mut start = 0;
    while let Some(pos) = line[start..].find(word) {
        let abs_pos = start + pos;
        f(abs_pos, word);
        start = abs_pos + word.len();
    }
}

/// Check if the match at `pos` with length `len` has word boundaries on both sides
/// (i.e., it's not part of a larger identifier like `trueValue` or `my_false`).
fn is_word_boundary(line: &str, pos: usize, len: usize) -> bool {
    let bytes = line.as_bytes();
    // Check char before
    if pos > 0 {
        let before = bytes[pos - 1];
        if before.is_ascii_alphanumeric() || before == b'_' {
            return false;
        }
    }
    // Check char after
    let after_pos = pos + len;
    if after_pos < bytes.len() {
        let after = bytes[after_pos];
        if after.is_ascii_alphanumeric() || after == b'_' {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lor_and_to_or() {
        let src = "if (a && b) return;\n";
        let m = LogicalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "&&");
        assert_eq!(mutants[0].replacement, "||");
    }

    #[test]
    fn test_lor_or_to_and() {
        let src = "if (a || b) return;\n";
        let m = LogicalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "||");
        assert_eq!(mutants[0].replacement, "&&");
    }

    #[test]
    fn test_lor_skips_bitwise() {
        // Single & should not be treated as &&
        let src = "var x = a & b;\n";
        let m = LogicalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_lcr_true_to_false() {
        let src = "var x = true;\n";
        let m = LogicalConstantReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "true");
        assert_eq!(mutants[0].replacement, "false");
    }

    #[test]
    fn test_lcr_false_to_true() {
        let src = "var x = false;\n";
        let m = LogicalConstantReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "false");
        assert_eq!(mutants[0].replacement, "true");
    }

    #[test]
    fn test_lcr_skips_identifier_parts() {
        let src = "var trueValue = 1;\n";
        let m = LogicalConstantReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // `true` in `trueValue` should not be mutated
        assert!(mutants.is_empty());
    }
}
