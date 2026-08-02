//! ROR — Relational Operator Replacement.
//!
//! Replaces each relational operator with every other relational operator.
//! Operators: `>`, `>=`, `<`, `<=`, `==`, `!=`.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// ROR — Relational Operator Replacement.
///
/// For each relational operator found in the source, produce a mutant that
/// replaces it with every other relational operator.
pub struct RelationalOperatorReplacement;

impl Mutator for RelationalOperatorReplacement {
    fn name(&self) -> &str {
        "Relational Operator Replacement"
    }

    fn code(&self) -> &str {
        "ROR"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        // (operator, [replacements])
        let op_map: &[(&str, &[&str])] = &[
            (">", &["<", ">=", "<=", "==", "!="]),
            (">=", &[">", "<", "<=", "==", "!="]),
            ("<", &[">", ">=", "<=", "==", "!="]),
            ("<=", &[">", ">=", "<", "==", "!="]),
            ("==", &[">", ">=", "<", "<=", "!="]),
            ("!=", &[">", ">=", "<", "<=", "=="]),
        ];

        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            // Scan for two-character operators first (>=, <=, ==, !=), then single (>  <).
            // We need to be careful not to match `=>` (arrow) or `->` (arrow).
            let chars: Vec<char> = clean.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let col = i + 1;
                if is_in_string_or_comment(line, col) {
                    i += 1;
                    continue;
                }

                // Check two-char operators
                if i + 1 < chars.len() {
                    let two = format!("{}{}", chars[i], chars[i + 1]);
                    // Skip => (fat arrow) and -> (arrow)
                    if two == "=>" || two == "->" {
                        i += 2;
                        continue;
                    }
                    if is_relational_op(&two) {
                        // Find replacements
                        let replacements = get_replacements(&two, op_map);
                        for &replacement in replacements {
                            mutants.push(Mutant::without_id(
                                file_path,
                                line_no,
                                col,
                                "ROR",
                                &two,
                                replacement,
                                format!("ROR: {} → {} at line {}", two, replacement, line_no),
                            ));
                        }
                        i += 2;
                        continue;
                    }
                }

                // Check single-char operators (> and <)
                let one = chars[i].to_string();
                if is_relational_op(&one) {
                    // Make sure it's not part of a two-char op we already handled
                    // (e.g., don't match `>` in `>=` — but we already skipped above)
                    // Also skip if followed by `=` (handled as two-char) or preceded by `<`/`>` (shift, generics)
                    // Skip if preceded by `<` (could be generics like List<int>)
                    let preceded_by = if i > 0 { Some(chars[i - 1]) } else { None };
                    let followed_by = if i + 1 < chars.len() {
                        Some(chars[i + 1])
                    } else {
                        None
                    };
                    // Skip `>=` and `<=` (already handled as two-char)
                    if followed_by == Some('=') {
                        i += 1;
                        continue;
                    }
                    // Skip if preceded by `<` or `>` (generics: List<int>, or >>)
                    if preceded_by == Some('<') || preceded_by == Some('>') {
                        i += 1;
                        continue;
                    }
                    // Skip if followed by `<` or `>` (generics or >>)
                    if followed_by == Some('<') || followed_by == Some('>') {
                        i += 1;
                        continue;
                    }

                    // Heuristic: skip `<` or `>` that look like generics.
                    // Generics pattern: `Identifier<Type>` — the `<` immediately
                    // follows an identifier char (no space) and is followed by
                    // a type name. Similarly `>` closing generics is immediately
                    // preceded by an identifier char and followed by non-operator.
                    if chars[i] == '<' && i > 0 {
                        let prev = chars[i - 1];
                        if prev.is_alphanumeric() || prev == '_' || prev == '>' {
                            // `List<` — likely generics, skip
                            i += 1;
                            continue;
                        }
                    }
                    if chars[i] == '>' && i > 0 {
                        let prev = chars[i - 1];
                        if prev.is_alphanumeric() || prev == '_' || prev == '>' {
                            // `int>` — likely closing generics, skip
                            i += 1;
                            continue;
                        }
                    }

                    let replacements = get_replacements(&one, op_map);
                    for &replacement in replacements {
                        mutants.push(Mutant::without_id(
                            file_path,
                            line_no,
                            col,
                            "ROR",
                            &one,
                            replacement,
                            format!("ROR: {} → {} at line {}", one, replacement, line_no),
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

fn is_relational_op(s: &str) -> bool {
    matches!(s, ">" | "<" | ">=" | "<=" | "==" | "!=")
}

fn get_replacements<'a>(op: &str, map: &'a [(&str, &[&str])]) -> &'a [&'a str] {
    for (key, replacements) in map {
        if *key == op {
            return replacements;
        }
    }
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ror_finds_mutations() {
        let src = "bool isGreater(int a, int b) => a > b;\n";
        let m = RelationalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // > produces 5 replacements
        assert_eq!(mutants.len(), 5);
        assert!(mutants.iter().all(|m| m.operator == "ROR"));
    }

    #[test]
    fn test_ror_skips_arrows() {
        let src = "var f = (x) => x + 1;\n";
        let m = RelationalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // => should not be treated as >=
        assert!(mutants.is_empty(), "should not match => arrow");
    }

    #[test]
    fn test_ror_skips_generics() {
        let src = "List<int> nums = [];\n";
        let m = RelationalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // <int> should not be treated as a relational operator
        assert!(mutants.is_empty(), "should not match generics <int>");
    }

    #[test]
    fn test_ror_equality() {
        let src = "if (x == 0) return;\n";
        let m = RelationalOperatorReplacement;
        let mutants = m.find_mutations(src, "test.dart");
        // == produces 5 replacements
        assert_eq!(mutants.len(), 5);
    }
}
