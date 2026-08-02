//! RVR — Return Value Replacement.
//!
//! Replaces return values with zero, empty, null, or false depending on the
//! return type. This tests whether the test suite checks the exact return value.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;
use regex::Regex;

/// RVR — Return Value Replacement.
///
/// For each `return expr;` statement, produce mutants that replace the
/// returned value with:
/// - `0` (for numeric returns)
/// - `""` (for string returns)
/// - `null` (for nullable returns)
/// - `false` (for boolean returns)
/// - `[]` (for list returns)
/// - `{}` (for map/set returns)
///
/// We generate one mutant per return statement, picking the most likely
/// replacement based on the expression type, plus a `0` fallback.
pub struct ReturnValueReplacement;

impl Mutator for ReturnValueReplacement {
    fn name(&self) -> &str {
        "Return Value Replacement"
    }

    fn code(&self) -> &str {
        "RVR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Match `return expr;` or `return expr;` on a single line.
        // We also handle `=> expr;` (arrow function returns).
        let re = Regex::new(r"\breturn\s+(?P<expr>[^;]+);").unwrap();
        let arrow_re = Regex::new(r"=>\s*(?P<expr>[^;]+);").unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Regular return statements
            for m in re.captures_iter(clean) {
                let expr = m.name("expr").map(|e| e.as_str()).unwrap_or("");
                let expr_trimmed = expr.trim();
                if expr_trimmed.is_empty() || expr_trimmed == ";" {
                    continue;
                }

                let full_start = m.get(0).unwrap().start();
                let col = full_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                // Generate replacements based on expression type
                for replacement in get_return_replacements(expr_trimmed) {
                    let original = format!("return {};", expr_trimmed);
                    let repl = format!("return {};", replacement);
                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "RVR",
                        &original,
                        &repl,
                        format!(
                            "RVR: return {} → return {} at line {}",
                            expr_trimmed, replacement, line_no
                        ),
                    ));
                }
            }

            // Arrow function returns: `=> expr;`
            for m in arrow_re.captures_iter(clean) {
                let expr = m.name("expr").map(|e| e.as_str()).unwrap_or("");
                let expr_trimmed = expr.trim();
                if expr_trimmed.is_empty() || expr_trimmed == ";" {
                    continue;
                }

                let full_start = m.get(0).unwrap().start();
                let col = full_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                for replacement in get_return_replacements(expr_trimmed) {
                    let original = format!("=> {};", expr_trimmed);
                    let repl = format!("=> {};", replacement);
                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "RVR",
                        &original,
                        &repl,
                        format!(
                            "RVR: => {} → => {} at line {}",
                            expr_trimmed, replacement, line_no
                        ),
                    ));
                }
            }
        }

        mutants
    }
}

/// Determine replacement values for a return expression based on its apparent type.
fn get_return_replacements(expr: &str) -> Vec<&'static str> {
    let mut replacements = Vec::new();

    // Boolean expressions
    if expr == "true" || expr == "false" {
        replacements.push(if expr == "true" { "false" } else { "true" });
        return replacements;
    }

    // Already null
    if expr == "null" {
        return replacements;
    }

    // String literals
    if (expr.starts_with('"') || expr.starts_with("'"))
        && (expr.ends_with('"') || expr.ends_with('\''))
    {
        replacements.push("\"\"");
        replacements.push("0");
        return replacements;
    }

    // List literals
    if expr.starts_with('[') {
        replacements.push("[]");
        replacements.push("0");
        return replacements;
    }

    // Map/Set literals
    if expr.starts_with('{') {
        replacements.push("{}");
        replacements.push("0");
        return replacements;
    }

    // Numeric expressions (contain digits or arithmetic operators)
    if expr.chars().any(|c| c.is_ascii_digit())
        || expr.contains('+')
        || expr.contains('-')
        || expr.contains('*')
        || expr.contains('/')
        || expr.contains('%')
    {
        replacements.push("0");
        replacements.push("null");
        return replacements;
    }

    // Default: try 0 and null (one of them should compile depending on return type)
    replacements.push("0");
    replacements.push("null");
    replacements.push("false");
    replacements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvr_numeric_return() {
        let src = "int add(int a, int b) {\n  return a + b;\n}\n";
        let m = ReturnValueReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
        assert!(mutants.iter().any(|m| m.replacement == "return 0;"));
    }

    #[test]
    fn test_rvr_boolean_return() {
        let src = "bool isEven(int n) {\n  return n % 2 == 0;\n}\n";
        let m = ReturnValueReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
    }

    #[test]
    fn test_rvr_arrow_function() {
        let src = "int add(int a, int b) => a + b;\n";
        let m = ReturnValueReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
        assert!(mutants.iter().any(|m| m.replacement.contains("=> 0;")));
    }

    #[test]
    fn test_rvr_string_return() {
        let src = "String greet() {\n  return \"hello\";\n}\n";
        let m = ReturnValueReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.iter().any(|m| m.replacement.contains("\"\"")));
    }
}
