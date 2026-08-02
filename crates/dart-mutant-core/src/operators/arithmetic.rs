//! Arithmetic operators: AOR (replacement), AOD (deletion), AOI (insertion).
//!
//! - **AOR** — Arithmetic Operator Replacement: `+` → `-`, `*` → `/`, `%` → `*`, etc.
//! - **AOD** — Arithmetic Operator Deletion: `a + b` → `a` (remove the operator and right operand).
//! - **AOI** — Arithmetic Operator Insertion: `a` → `a + 1` (insert `+ 1` after a variable/number).

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;
use regex::Regex;

/// AOR — Arithmetic Operator Replacement.
///
/// Replaces each binary arithmetic operator with every other arithmetic operator.
/// Operators: `+`, `-`, `*`, `/`, `%`.
pub struct ArithmeticOperatorReplacement;

impl Mutator for ArithmeticOperatorReplacement {
    fn name(&self) -> &str {
        "Arithmetic Operator Replacement"
    }

    fn code(&self) -> &str {
        "AOR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();
        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Find all arithmetic operators in the line.
            // We scan for operator chars between non-whitespace (binary operators).
            for (col_idx, ch) in clean.char_indices() {
                let col = col_idx + 1;
                if !is_arithmetic_op_char(ch) {
                    continue;
                }
                if is_in_string_or_comment(line, col) {
                    continue;
                }
                // Check it's a binary operator (has something before and after).
                if !is_binary_op(clean, col_idx) {
                    continue;
                }
                let op_str = ch.to_string();
                let replacements: &[&str] = match op_str.as_str() {
                    "+" => &["-", "*", "/", "%"],
                    "-" => &["+", "*", "/", "%"],
                    "*" => &["+", "-", "/", "%"],
                    "/" => &["+", "-", "*", "%"],
                    "%" => &["+", "-", "*", "/"],
                    _ => continue,
                };
                for &replacement in replacements {
                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "AOR",
                        &op_str,
                        replacement,
                        format!("AOR: {} → {} at line {}", op_str, replacement, line_no),
                    ));
                }
            }
        }
        mutants
    }
}

/// AOD — Arithmetic Operator Deletion.
///
/// Removes the operator and the right-hand operand: `a + b` → `a`.
/// This tests whether the right operand's contribution is verified by tests.
pub struct ArithmeticOperatorDeletion;

impl Mutator for ArithmeticOperatorDeletion {
    fn name(&self) -> &str {
        "Arithmetic Operator Deletion"
    }

    fn code(&self) -> &str {
        "AOD"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Regex to find binary arithmetic expressions: operand OP operand
        // We capture the left operand and the operator (to produce "left operand" only).
        let re =
            Regex::new(r"(?P<left>[\w\)\]\}\.]+)\s*(?P<op>[+\-*/%])\s*(?P<right>[\w\(\[\{\.]+)")
                .unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            for m in re.captures_iter(clean) {
                let full = m.get(0).unwrap();
                let op_pos = full.start();
                let col = op_pos + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }
                // The "original" is the full match "left OP right"
                let original = full.as_str();
                // The "replacement" is just the left operand (the captured "left" group)
                let left = m.name("left").map(|l| l.as_str()).unwrap_or("");
                // Only do AOD if the right operand looks like a simple expression
                // (to avoid producing invalid code)
                let right = m.name("right").map(|r| r.as_str()).unwrap_or("");
                if right.is_empty() || left.is_empty() {
                    continue;
                }
                // Skip if this looks like a unary minus (e.g., at start of expression)
                if op_pos == 0 {
                    continue;
                }

                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "AOD",
                    original,
                    left,
                    format!(
                        "AOD: remove `{}` (keep `{}`) at line {}",
                        original, left, line_no
                    ),
                ));
            }
        }
        mutants
    }
}

/// AOI — Arithmetic Operator Insertion.
///
/// Inserts `+ 1` after a variable or number: `a` → `a + 1`.
/// This tests whether the exact value is checked by tests.
pub struct ArithmeticOperatorInsertion;

impl Mutator for ArithmeticOperatorInsertion {
    fn name(&self) -> &str {
        "Arithmetic Operator Insertion"
    }

    fn code(&self) -> &str {
        "AOI"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Find standalone identifiers and numbers that could be operands.
        // We look for patterns like: identifier or number followed by whitespace/operator.
        let re = Regex::new(r"\b(?P<operand>[a-zA-Z_]\w*|\d+\.?\d*)\b").unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            for m in re.find_iter(clean) {
                let pos = m.start();
                let col = pos + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                let operand = m.as_str();
                // Skip keywords
                if is_dart_keyword(operand) {
                    continue;
                }
                // Skip if the operand is followed by `(` (function call) or `.` (member access)
                // or `[` (index) — inserting `+ 1` there would break syntax.
                let after = &clean[m.end()..];
                let after_trimmed = after.trim_start();
                if after_trimmed.starts_with('(')
                    || after_trimmed.starts_with('.')
                    || after_trimmed.starts_with('[')
                {
                    continue;
                }
                // Skip if the operand is preceded by a `.` (it's a member access)
                let before = &clean[..pos];
                if before.trim_end().ends_with('.') {
                    continue;
                }
                // Skip if preceded by an operator (this is a right operand, already handled by AOR)
                // We want standalone left operands.

                let original = operand.to_string();
                let replacement = format!("{} + 1", operand);
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "AOI",
                    &original,
                    &replacement,
                    format!("AOI: `{}` → `{} + 1` at line {}", operand, operand, line_no),
                ));
            }
        }
        mutants
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_arithmetic_op_char(ch: char) -> bool {
    matches!(ch, '+' | '-' | '*' | '/' | '%')
}

/// Check if the character at `pos` in `line` is a binary operator (has
/// non-whitespace, non-operator characters on both sides).
fn is_binary_op(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    if pos == 0 || pos >= bytes.len() {
        return false;
    }
    // Look backwards for a non-whitespace char
    let mut i = pos;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let before_char = bytes[i - 1] as char;
    // The char before must be an operand-ending character
    if !is_operand_char(before_char) {
        return false;
    }
    // Look forwards for a non-whitespace char
    let mut j = pos + 1;
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j >= bytes.len() {
        return false;
    }
    let after_char = bytes[j] as char;
    // The char after must be an operand-starting character (but not another operator)
    if is_arithmetic_op_char(after_char) {
        return false;
    }
    // Allow ( for grouped expressions, ! for negation, etc.
    is_operand_start_char(after_char) || after_char == '(' || after_char == '!'
}

fn is_operand_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == ')' || ch == ']' || ch == '}' || ch == '.'
}

fn is_operand_start_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '(' || ch == '['
}

fn is_dart_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "default"
            | "break"
            | "continue"
            | "return"
            | "var"
            | "final"
            | "const"
            | "late"
            | "class"
            | "enum"
            | "mixin"
            | "extension"
            | "typedef"
            | "void"
            | "dynamic"
            | "get"
            | "set"
            | "factory"
            | "abstract"
            | "interface"
            | "implements"
            | "extends"
            | "with"
            | "new"
            | "this"
            | "super"
            | "true"
            | "false"
            | "null"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "rethrow"
            | "assert"
            | "in"
            | "is"
            | "as"
            | "covariant"
            | "show"
            | "hide"
            | "deferred"
            | "library"
            | "import"
            | "export"
            | "part"
            | "of"
            | "on"
            | "sync"
            | "async"
            | "await"
            | "yield"
            | "operator"
            | "static"
            | "external"
            | "sealed"
            | "base"
            | "when"
            | "required"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aor_finds_mutations() {
        let src = "int add(int a, int b) => a + b;\n";
        let m = ArithmeticOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty(), "should find + operator");
        // + should produce 4 replacements (-, *, /, %)
        assert_eq!(mutants.len(), 4);
        assert!(mutants.iter().all(|m| m.operator == "AOR"));
    }

    #[test]
    fn test_aor_multiple_operators() {
        let src = "var x = a + b * c;\n";
        let m = ArithmeticOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // + gives 4, * gives 4 = 8 total
        assert_eq!(mutants.len(), 8);
    }

    #[test]
    fn test_aor_skips_strings() {
        let src = "var s = \"a + b\";\n";
        let m = ArithmeticOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty(), "should not mutate inside strings");
    }

    #[test]
    fn test_aor_skips_comments() {
        let src = "var x = 1; // a + b\n";
        let m = ArithmeticOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // The + in the comment should be skipped
        assert!(mutants.is_empty(), "should not mutate inside comments");
    }

    #[test]
    fn test_aod_finds_mutations() {
        let src = "var x = a + b;\n";
        let m = ArithmeticOperatorDeletion;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
        assert_eq!(mutants[0].original, "a + b");
        assert_eq!(mutants[0].replacement, "a");
    }

    #[test]
    fn test_aoi_finds_mutations() {
        let src = "var x = a + b;\n";
        let m = ArithmeticOperatorInsertion;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
        // Should insert +1 after `a` and `b`
        assert!(mutants.iter().any(|m| m.replacement == "a + 1"));
        assert!(mutants.iter().any(|m| m.replacement == "b + 1"));
    }

    #[test]
    fn test_aoi_skips_keywords() {
        let src = "if (x > 0) return;\n";
        let m = ArithmeticOperatorInsertion;
        let mutants = m.find_mutations(src, "test.dart");
        // Should not insert +1 after `if` or `return`
        assert!(!mutants.iter().any(|m| m.original == "if"));
        assert!(!mutants.iter().any(|m| m.original == "return"));
    }
}
