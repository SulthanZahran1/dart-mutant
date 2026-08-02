//! Conditional operators: COR (conditional replacement) and negate.
//!
//! - **COR** — Conditional Operator Replacement: `if (x)` → `if (!x)`, negates the condition.
//! - **negate** — Negate the condition of `if`, `while`, `do-while` by wrapping in `!(...)`.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;
use regex::Regex;

/// COR — Conditional Operator Replacement.
///
/// Negates the condition of `if`, `while`, and `do-while` statements by
/// wrapping the condition in `!(...)`.
pub struct ConditionalOperatorReplacement;

impl Mutator for ConditionalOperatorReplacement {
    fn name(&self) -> &str {
        "Conditional Operator Replacement"
    }

    fn code(&self) -> &str {
        "COR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Match if/while/do-while conditions.
        // Pattern: keyword (condition)
        let re = Regex::new(r"(?P<kw>\bif|\bwhile)\s*\((?P<cond>[^{}]+?)\)").unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            for m in re.captures_iter(clean) {
                let kw = m.name("kw").map(|k| k.as_str()).unwrap_or("");
                let cond = m.name("cond").map(|c| c.as_str()).unwrap_or("");
                if cond.is_empty() {
                    continue;
                }

                let full_match_start = m.get(0).unwrap().start();
                let col = full_match_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                let original = format!("{}({})", kw, cond);
                let replacement = format!("{}(!({}))", kw, cond);
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "COR",
                    &original,
                    &replacement,
                    format!("COR: negate condition of `{}` at line {}", kw, line_no),
                ));
            }
        }

        // Also handle `do { } while (cond);`
        let do_while_re = Regex::new(r"\bwhile\s*\((?P<cond>[^{}]+?)\)").unwrap();
        for line in source.lines() {
            let clean = strip_comment(line);
            // This is redundant with above but catches `while` on its own line after `do { }`.
            for m in do_while_re.captures_iter(clean) {
                let cond = m.name("cond").map(|c| c.as_str()).unwrap_or("");
                if cond.is_empty() {
                    continue;
                }
                let full_match_start = m.get(0).unwrap().start();
                let col = full_match_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }
                // Skip if we already captured this as an if/while (the first regex handles while too)
                // Check if this match was already captured by the first regex
                let full = m.get(0).unwrap().as_str();
                // The first regex already handles `while (...)` — we skip duplicates by checking
                // if this line had a while that was already processed.
                // Since both regexes match `while (...)`, we only add mutants from the first regex.
                // This is a no-op for do-while unless the `while` is on a separate line.
                let _ = full;
            }
        }

        mutants
    }
}

/// Negate — a simpler variant that just negates boolean expressions.
///
/// Wraps the expression in `!(...)`. This is used for standalone boolean
/// expressions (not just if/while conditions).
pub struct NegateOperator;

impl Mutator for NegateOperator {
    fn name(&self) -> &str {
        "Negate"
    }

    fn code(&self) -> &str {
        "NEGATE"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Find return statements with a boolean expression: `return expr;`
        let re = Regex::new(r"\breturn\s+(?P<expr>[^;]+);").unwrap();
        // Also handle arrow functions: `=> expr;` where expr is boolean
        let arrow_re = Regex::new(r"=>\s*(?P<expr>[^;]+);").unwrap();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            for m in re.captures_iter(clean) {
                let expr = m.name("expr").map(|e| e.as_str()).unwrap_or("");
                if expr.is_empty() {
                    continue;
                }
                // Only negate if the expression looks like a boolean (contains comparison/logical operators)
                let expr_trimmed = expr.trim();
                if !looks_boolean(expr_trimmed) {
                    continue;
                }

                let full_start = m.get(0).unwrap().start();
                let col = full_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                let original = format!("return {};", expr_trimmed);
                let replacement = format!("return !({});", expr_trimmed);
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "NEGATE",
                    &original,
                    &replacement,
                    format!("Negate: negate return expression at line {}", line_no),
                ));
            }

            // Handle arrow functions (regex hoisted out of the loop)
            for m in arrow_re.captures_iter(clean) {
                let expr = m.name("expr").map(|e| e.as_str()).unwrap_or("");
                if expr.is_empty() {
                    continue;
                }
                let expr_trimmed = expr.trim();
                if !looks_boolean(expr_trimmed) {
                    continue;
                }
                let full_start = m.get(0).unwrap().start();
                let col = full_start + 1;
                if is_in_string_or_comment(line, col) {
                    continue;
                }

                let original = format!("=> {};", expr_trimmed);
                let replacement = format!("=> !({});", expr_trimmed);
                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "NEGATE",
                    &original,
                    &replacement,
                    format!("Negate: negate arrow expression at line {}", line_no),
                ));
            }
        }

        mutants
    }
}

fn looks_boolean(expr: &str) -> bool {
    expr.contains("==")
        || expr.contains("!=")
        || expr.contains(">")
        || expr.contains("<")
        || expr.contains("&&")
        || expr.contains("||")
        || expr.contains("!")
        || expr.trim() == "true"
        || expr.trim() == "false"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cor_if() {
        let src = "if (x > 0) return x;\n";
        let m = ConditionalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert!(mutants[0].replacement.contains("!("));
    }

    #[test]
    fn test_cor_while() {
        let src = "while (x < 10) { x++; }\n";
        let m = ConditionalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
    }

    #[test]
    fn test_cor_skips_strings() {
        let src = "var s = \"if (x)\";\n";
        let m = ConditionalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_negate_boolean_return() {
        let src = "bool isEven(int n) => n % 2 == 0;\n";
        let m = NegateOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(!mutants.is_empty());
        assert!(mutants[0].replacement.contains("!("));
    }
}
