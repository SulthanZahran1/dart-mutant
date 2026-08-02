//! Dart null-safety operators: NullSafety (`??`), NullAssert (`!`), OptionalChaining (`?.`).
//!
//! - **NullSafety** — `a ?? b` → `a` (remove the null fallback).
//! - **NullAssert** — `a!` → `a` (remove the null assertion).
//! - **OptionalChaining** — `a?.b` → `a.b` (remove safe member access).

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// NullSafety — Remove the `??` null-coalescing operator.
///
/// `a ?? b` → `a` (removes the fallback value, tests if the suite catches
/// missing null handling).
pub struct NullSafetyOperator;

impl Mutator for NullSafetyOperator {
    fn name(&self) -> &str {
        "NullSafety"
    }

    fn code(&self) -> &str {
        "NullSafety"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Find `??` operator (not `??=` which is a null-aware assignment)
            if let Some(pos) = find_operator(clean, "??") {
                let col = pos + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }
                // Check it's not `??=` (null-aware assignment)
                let after = &clean[pos + 2..];
                if after.trim_start().starts_with('=') {
                    continue;
                }

                // Extract the left operand (everything before ??, trimmed)
                let _left = clean[..pos].trim();
                let _right = clean[pos + 2..].trim();
                // The original is `left ?? right`, replacement is just `left`
                // But we need to be careful about what `left` and `right` are.
                // For simplicity, the "original" is `??` and replacement is removing it.
                // We set original to the full `a ?? b` expression and replacement to `a`.

                // Find the left operand: scan backwards from `??` to find the start of the expression.
                let left_operand = extract_left_operand(&clean[..pos]);
                let right_operand = extract_right_operand(&clean[pos + 2..]);

                if !left_operand.is_empty() && !right_operand.is_empty() {
                    let original = format!("{} ?? {}", left_operand, right_operand);
                    let replacement = left_operand.to_string();
                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "NullSafety",
                        &original,
                        &replacement,
                        format!(
                            "NullSafety: remove `?? {}` (null fallback) at line {}",
                            right_operand, line_no
                        ),
                    ));
                }
            }
        }

        mutants
    }
}

/// NullAssert — Remove the `!` null assertion operator.
///
/// `a!` → `a` (removes the force-unwrap, tests if the suite catches
/// unsafe null access).
pub struct NullAssertOperator;

impl Mutator for NullAssertOperator {
    fn name(&self) -> &str {
        "NullAssert"
    }

    fn code(&self) -> &str {
        "NullAssert"
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

                // Look for `!` that is a null assertion (preceded by identifier/`)`/`]`)
                // and NOT followed by `=` (that would be `!=`) or `=` before it (that would be `==`)
                // and NOT preceded by `<` (that could be a generic)
                if chars[i] == '!' {
                    // Skip `!=` (already handled by ROR)
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        i += 2;
                        continue;
                    }
                    // Must be preceded by an operand character (identifier, ), ], ., etc.)
                    if i == 0 {
                        i += 1;
                        continue;
                    }
                    let before = chars[i - 1];
                    if !(before.is_alphanumeric()
                        || before == '_'
                        || before == ')'
                        || before == ']'
                        || before == '.')
                    {
                        i += 1;
                        continue;
                    }
                    // Must NOT be followed by an identifier (that would be `!variable` which is logical not)
                    // Actually, `a!` is null assertion. `!a` is logical not.
                    // The key: null assertion `!` comes AFTER an operand.
                    // Logical not `!` comes BEFORE an operand.
                    // We already check `before` is an operand char, so this is null assertion.
                    // But we should also make sure it's not `a != b` (handled above).

                    // Check it's not followed by `.` followed by identifier (that's `a!.b` — we can still remove `!`)
                    // Actually `a!.b` → `a.b` is exactly what we want.

                    // Extract the operand before `!`
                    let left_operand = extract_left_operand(&clean[..i]);
                    if !left_operand.is_empty() {
                        let original = format!("{}!", left_operand);
                        let replacement = left_operand.to_string();
                        mutants.push(Mutant::without_id(
                            file_path,
                            line_no,
                            col,
                            "NullAssert",
                            &original,
                            &replacement,
                            format!(
                                "NullAssert: remove `!` (null assertion) at line {}",
                                line_no
                            ),
                        ));
                    }
                    i += 1;
                    continue;
                }

                i += 1;
            }
        }

        mutants
    }
}

/// OptionalChaining — Replace `?.` with `.` (remove safe member access).
///
/// `a?.b` → `a.b` (removes the safe access, tests if the suite catches
/// null propagation gaps).
pub struct OptionalChainingOperator;

impl Mutator for OptionalChainingOperator {
    fn name(&self) -> &str {
        "OptionalChaining"
    }

    fn code(&self) -> &str {
        "OptionalChaining"
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

                // Look for `?.` (optional chaining)
                if chars[i] == '?' && i + 1 < chars.len() && chars[i + 1] == '.' {
                    // Make sure it's not `?.[` (optional index) or `?(...)` (optional call)
                    // Actually `?.` can be followed by `[` or `(` in Dart 3 patterns, but
                    // for mutation purposes, replacing `?.` with `.` should still work.
                    // Must be preceded by an operand character
                    if i == 0 {
                        i += 2;
                        continue;
                    }
                    let before = chars[i - 1];
                    if !(before.is_alphanumeric()
                        || before == '_'
                        || before == ')'
                        || before == ']')
                    {
                        i += 2;
                        continue;
                    }

                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "OptionalChaining",
                        "?.",
                        ".",
                        format!("OptionalChaining: ?. → . at line {}", line_no),
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the byte position of the first occurrence of `op` in `line`,
/// skipping string literals.
fn find_operator(line: &str, op: &str) -> Option<usize> {
    let mut in_string = false;
    let mut quote_char = b'"';
    let mut escaped = false;
    let bytes = line.as_bytes();
    let op_bytes = op.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        let b = bytes[i];
        if b == b'\\' && in_string {
            escaped = true;
            i += 1;
            continue;
        }
        if in_string {
            if b == quote_char {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            in_string = true;
            quote_char = b;
            i += 1;
            continue;
        }
        if i + op_bytes.len() <= bytes.len() && &bytes[i..i + op_bytes.len()] == op_bytes {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract the left operand of a binary operator by scanning backwards.
/// This is a heuristic: we look for the longest valid identifier/expression
/// ending at the end of `text`.
fn extract_left_operand(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    // Scan backwards collecting identifier chars, dots, brackets, parens.
    let chars: Vec<char> = trimmed.chars().collect();
    let mut end = chars.len();
    let mut start = end;

    // Skip trailing whitespace
    while start > 0 && chars[start - 1].is_whitespace() {
        start -= 1;
        end = start;
    }

    // Collect identifier chars and chained accesses (.method, [index], (args))
    // We scan backwards allowing: alnum, _, ., ), ], }, and whitespace between them.
    let mut depth = 0i32;
    while start > 0 {
        let c = chars[start - 1];
        if c.is_alphanumeric() || c == '_' || c == '.' {
            start -= 1;
        } else if c == ')' || c == ']' || c == '}' {
            depth += 1;
            start -= 1;
        } else if c == '(' || c == '[' || c == '{' {
            if depth > 0 {
                depth -= 1;
                start -= 1;
            } else {
                break;
            }
        } else if c.is_whitespace() && depth > 0 {
            start -= 1;
        } else {
            break;
        }
    }

    if start < end {
        chars[start..end].iter().collect()
    } else {
        String::new()
    }
}

/// Extract the right operand of a binary operator by scanning forwards.
fn extract_right_operand(text: &str) -> String {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }
    // Take the first identifier/expression
    let end = trimmed
        .find([';', ',', ')', ']', '}', '\n'])
        .unwrap_or(trimmed.len());
    trimmed[..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_safety_removes_fallback() {
        let src = "var x = a ?? b;\n";
        let m = NullSafetyOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert!(mutants[0].replacement.contains("a"));
        assert!(!mutants[0].replacement.contains("??"));
    }

    #[test]
    fn test_null_safety_skips_null_aware_assignment() {
        let src = "a ??= b;\n";
        let m = NullSafetyOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_null_assert_removes_bang() {
        let src = "var x = nullable!;\n";
        let m = NullAssertOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].replacement, "nullable");
    }

    #[test]
    fn test_null_assert_skips_not_equal() {
        let src = "if (a != b) return;\n";
        let m = NullAssertOperator;
        let mutants = m.find_mutations(src, "test.dart");
        // `!=` should not be treated as null assertion
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_optional_chaining() {
        let src = "var x = obj?.value;\n";
        let m = OptionalChainingOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, "?.");
        assert_eq!(mutants[0].replacement, ".");
    }
}
