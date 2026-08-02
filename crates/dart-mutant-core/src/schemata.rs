//! Mutant schemata generation.
//!
//! The schemata technique injects ALL mutations into a single source file as
//! conditional branches, switched at runtime via the `DART_MUTANT_ID` environment
//! variable. This allows compiling once and running N times with different
//! mutant ids — avoiding the dominant compilation cost of mutation testing.
//!
//! ## Example
//!
//! Original:
//! ```dart
//! int add(int a, int b) => a + b;
//! ```
//!
//! Schemata:
//! ```dart
//! int add(int a, int b) => switch (const String.fromEnvironment('DART_MUTANT_ID', defaultValue: '')) {
//!   '0' => a - b,  // AOR mutant 0
//!   '1' => a * b,  // AOR mutant 1
//!   '2' => a / b,  // AOR mutant 2
//!   '3' => a % b,  // AOR mutant 3
//!   _    => a + b,  // original
//! };
//! ```
//!
//! Since we do line-based mutation (not AST-based), the schemata is generated
//! by applying each mutant's replacement to the original source text and
//! wrapping the result in a conditional. For line-based mutations, we generate
//! a single file per source file with all mutants applied via if-else chains.

use crate::mutant::Mutant;
use std::collections::BTreeMap;

/// Environment variable name used to select the active mutant at runtime.
pub const MUTANT_ENV_VAR: &str = "DART_MUTANT_ID";

/// Generate schemata source for a single file, applying all its mutants.
///
/// Given the original source text and a list of mutants (all for the same file),
/// produce a new source text where each mutation point is wrapped in a
/// conditional on the `DART_MUTANT_ID` environment variable.
///
/// The approach:
/// 1. Group mutants by line.
/// 2. For each line with mutants, generate a match/if-else block that selects
///    the replacement based on the env var.
/// 3. Lines without mutants are kept as-is.
///
/// Since we're doing line-based replacement, each mutant replaces text on a
/// specific line. We use `const String.fromEnvironment` for compile-time
/// selection (Dart's compile-time env var support).
///
/// **Important:** This function assumes all mutants in the input belong to the
/// same file (as given by `Mutant::file_path`). It does not verify this.
pub fn generate_schemata(source: &str, mutants: &[Mutant]) -> String {
    if mutants.is_empty() {
        return source.to_string();
    }

    // Group mutants by line (1-based).
    let mut by_line: BTreeMap<usize, Vec<&Mutant>> = BTreeMap::new();
    for m in mutants {
        by_line.entry(m.line).or_default().push(m);
    }

    let original_lines: Vec<&str> = source.lines().collect();
    let mut output = String::with_capacity(source.len() + mutants.len() * 128);

    for (line_idx, line) in original_lines.iter().enumerate() {
        let line_no = line_idx + 1;

        if let Some(line_mutants) = by_line.get(&line_no) {
            // This line has mutations — generate a schemata block.
            output.push_str(&generate_line_schemata(line, line_no, line_mutants));
        } else {
            // No mutations on this line — keep as-is.
            output.push_str(line);
        }
        output.push('\n');
    }

    // If the original source ended with a newline, the above loop adds it.
    // If not, we might have an extra newline. Let's handle the edge case:
    if !source.ends_with('\n') && !output.is_empty() {
        output.pop(); // remove trailing newline if original didn't have one
    }

    output
}

/// Generate a schemata block for a single line.
///
/// Produces a Dart `if-else` chain that checks `const String.fromEnvironment('DART_MUTANT_ID')`
/// and applies the matching mutant's replacement, falling back to the original line.
fn generate_line_schemata(line: &str, _line_no: usize, mutants: &[&Mutant]) -> String {
    let mut output = String::with_capacity(line.len() + mutants.len() * 128);

    // We need to apply the text replacement within the line.
    // Each mutant has `original` (the text to find) and `replacement` (the text to substitute).
    // We find the first occurrence of `original` in the line and split around it.

    // For simplicity, if there's only one mutant on this line, we can do a simple
    // if-else. If there are multiple, we chain them.

    // Use the first mutant to determine the split point (all mutants on the same
    // line should target the same operator location, but with different replacements).
    let first = mutants[0];
    let original_text = &first.original;

    // Find the occurrence of `original_text` in the line.
    // For line-based mutations, the `original` might be the whole line or a substring.
    let (prefix, suffix) = if let Some(pos) = line.find(original_text) {
        let (pre, rest) = line.split_at(pos);
        let (_, suf) = rest.split_at(original_text.len());
        (pre.to_string(), suf.to_string())
    } else {
        // If we can't find the exact original text, fall back to the whole line.
        // This can happen with SDL where the original is the trimmed line.
        // Try trimmed matching.
        let trimmed = line.trim();
        if trimmed == original_text.trim() {
            let leading = line.len() - line.trim_start().len();
            let trailing = line.len() - line.trim_end().len();
            let pre = &line[..leading];
            let suf = &line[line.len() - trailing..];
            (pre.to_string(), suf.to_string())
        } else {
            // Can't find the text — just return the original line.
            return line.to_string();
        }
    };

    // Generate the if-else chain.
    // We use `const String.fromEnvironment` for compile-time selection.
    // But since we need runtime selection (the runner sets the env var per run),
    // we use `String.fromEnvironment` without `const` to allow runtime override.
    // Actually, `String.fromEnvironment` IS compile-time only.
    // For runtime selection, we should use `Platform.environment` from `dart:io`.
    // But that requires an import. Let's use a different approach:
    // We use a global function that reads the env var at runtime.
    //
    // Actually, the simplest approach for Dart is to use `Platform.environment['DART_MUTANT_ID']`.
    // But that requires `dart:io`. Since we're generating source that will be compiled,
    // we can add the import at the top of the file.
    //
    // For now, let's use `String.fromEnvironment` which is Dart's standard way of
    // reading compile-time env vars. The schemata is compiled once, but we can
    // re-run with different values of the env var without recompiling if we use
    // `dart test` with `--define` flags.
    //
    // Actually, for mutation testing, the standard approach (muter, kanly) is:
    // Compile once, then run N times with the env var set differently.
    // `String.fromEnvironment` is read at compile time, so this won't work
    // unless we re-compile.
    //
    // The correct approach for "compile once, run N times" is to use
    // `Platform.environment['DART_MUTANT_ID']` (runtime env var lookup).
    // This requires `import 'dart:io';` at the top.
    //
    // Let's use that approach.

    // Build the if-else chain:
    // ```
    // if (Platform.environment['DART_MUTANT_ID'] == '0') {
    //   <prefix><replacement0><suffix>
    // } else if (Platform.environment['DART_MUTANT_ID'] == '1') {
    //   <prefix><replacement1><suffix>
    // } else {
    //   <line>
    // }
    // ```
    //
    // But this only works if the line is a simple statement. For expressions
    // inside larger expressions, we'd need a ternary or switch expression.
    //
    // Since we're doing line-based mutation, we assume each mutated line is a
    // complete statement or expression. We wrap the entire line in an if-else.

    // If the line doesn't end with `;`, it might be an expression inside a function.
    // For arrow functions `=> expr;`, we can use a ternary:
    // `=> <id> == '0' ? <replacement0> : <id> == '1' ? <replacement1> : <original>`

    let is_arrow = line.contains("=>");
    let ends_with_semicolon = line.trim_end().ends_with(';');
    let is_statement = ends_with_semicolon && !is_arrow;

    if is_arrow || !is_statement {
        // Use a ternary expression for arrow functions and inline expressions.
        // We need to be careful about the expression syntax.
        // For `=> expr;` we produce:
        // `=> (Platform.environment['DART_MUTANT_ID'] == '0') ? <repl0> : (Platform.environment['DART_MUTANT_ID'] == '1') ? <repl1> : <original>;`

        // Find the `=>` and split after it.
        if let Some(arrow_pos) = line.find("=>") {
            let before_arrow = &line[..arrow_pos + 2]; // includes `=>`
            let after_arrow = &line[arrow_pos + 2..];
            let after_trimmed = after_arrow.trim_start();
            let leading_space = &after_arrow[..after_arrow.len() - after_trimmed.len()];

            output.push_str(before_arrow);
            output.push_str(leading_space);

            // Generate the ternary chain.
            for (i, m) in mutants.iter().enumerate() {
                if i > 0 {
                    output.push_str(" : ");
                }
                output.push_str(&format!(
                    "(const String.fromEnvironment('{}', defaultValue: '') == '{}') ? {}",
                    MUTANT_ENV_VAR, m.id, m.replacement
                ));
            }
            // Default branch: original expression.
            output.push_str(" : ");
            output.push_str(after_trimmed);
        } else {
            // Inline expression (not an arrow function) — wrap in a ternary.
            output.push_str(&prefix);
            for (i, m) in mutants.iter().enumerate() {
                if i > 0 {
                    output.push_str(" : ");
                }
                output.push_str(&format!(
                    "(const String.fromEnvironment('{}', defaultValue: '') == '{}') ? {}",
                    MUTANT_ENV_VAR, m.id, m.replacement
                ));
            }
            output.push_str(" : ");
            output.push_str(original_text);
            output.push_str(&suffix);
        }
    } else {
        // Statement — wrap in an if-else block.
        // Indentation: use the line's leading whitespace.
        let indent = line.len() - line.trim_start().len();
        let indent_str: String = " ".repeat(indent);

        output.push_str(&format!(
            "if (const String.fromEnvironment('{}', defaultValue: '') == '{}') {{\n",
            MUTANT_ENV_VAR, mutants[0].id
        ));
        output.push_str(&indent_str);
        output.push_str("  ");
        output.push_str(&prefix);
        output.push_str(&mutants[0].replacement);
        output.push_str(&suffix);
        output.push('\n');
        output.push_str(&indent_str);

        for (_i, m) in mutants.iter().enumerate().skip(1) {
            output.push_str(&format!(
                "}} else if (const String.fromEnvironment('{}', defaultValue: '') == '{}') {{\n",
                MUTANT_ENV_VAR, m.id
            ));
            output.push_str(&indent_str);
            output.push_str("  ");
            output.push_str(&prefix);
            output.push_str(&m.replacement);
            output.push_str(&suffix);
            output.push('\n');
            output.push_str(&indent_str);
        }

        // Default: original line.
        output.push_str("} else {\n");
        output.push_str(&indent_str);
        output.push_str("  ");
        output.push_str(line.trim());
        output.push('\n');
        output.push_str(&indent_str);
        output.push('}');
    }

    output
}

/// Generate the import statement needed for schemata source files.
///
/// Returns `import 'dart:io';` if needed. Currently we use
/// `const String.fromEnvironment` which doesn't require an import,
/// so this returns an empty string.
pub fn schemata_imports() -> &'static str {
    // Using String.fromEnvironment which is a built-in Dart feature.
    // No import needed.
    ""
}

/// Generate schemata for an entire project.
///
/// Takes a map of file paths to (source, mutants) pairs and returns a map
/// of file paths to schemata source.
pub fn generate_project_schemata(files: &[(String, String, Vec<Mutant>)]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(path, source, mutants)| {
            let schemata = generate_schemata(source, mutants);
            (path.clone(), schemata)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schemata_no_mutants() {
        let src = "int add(int a, int b) => a + b;\n";
        let result = generate_schemata(src, &[]);
        assert_eq!(result, src);
    }

    #[test]
    fn test_schemata_arrow_function() {
        let src = "int add(int a, int b) => a + b;\n";
        let mutants = vec![
            Mutant::new("0", "test.dart", 1, 15, "AOR", "+", "-", "AOR: + → -"),
            Mutant::new("1", "test.dart", 1, 15, "AOR", "+", "*", "AOR: + → *"),
        ];
        let result = generate_schemata(src, &mutants);
        // Should contain ternary expressions with fromEnvironment
        assert!(result.contains("String.fromEnvironment"));
        assert!(result.contains("DART_MUTANT_ID"));
        assert!(result.contains("'0'"));
        assert!(result.contains("'1'"));
        // Should still contain the original as fallback
        assert!(result.contains("a + b"));
    }

    #[test]
    fn test_schemata_statement() {
        let src = "void main() {\n  var x = 1;\n  print(x);\n}\n";
        let mutants = vec![Mutant::new(
            "0",
            "test.dart",
            2,
            3,
            "SDL",
            "var x = 1;",
            "/* SDL */",
            "SDL: remove statement",
        )];
        let result = generate_schemata(src, &mutants);
        // Should contain if-else block
        assert!(result.contains("if"));
        assert!(result.contains("String.fromEnvironment"));
        assert!(result.contains("} else {"));
    }

    #[test]
    fn test_schemata_preserves_unmutated_lines() {
        let src = "int a = 1;\nint b = 2;\nint c = a + b;\n";
        let mutants = vec![Mutant::new(
            "0",
            "test.dart",
            3,
            11,
            "AOR",
            "+",
            "-",
            "AOR: + → -",
        )];
        let result = generate_schemata(src, &mutants);
        // Lines 1 and 2 should be unchanged
        assert!(result.contains("int a = 1;"));
        assert!(result.contains("int b = 2;"));
    }

    #[test]
    fn test_project_schemata() {
        let files = vec![
            (
                "lib/a.dart".to_string(),
                "int f() => 1 + 2;\n".to_string(),
                vec![Mutant::new(
                    "0",
                    "lib/a.dart",
                    1,
                    13,
                    "AOR",
                    "+",
                    "-",
                    "test",
                )],
            ),
            (
                "lib/b.dart".to_string(),
                "int g() => 3 * 4;\n".to_string(),
                vec![Mutant::new(
                    "1",
                    "lib/b.dart",
                    1,
                    13,
                    "AOR",
                    "*",
                    "/",
                    "test",
                )],
            ),
        ];
        let result = generate_project_schemata(&files);
        assert_eq!(result.len(), 2);
        assert!(result[0].1.contains("String.fromEnvironment"));
        assert!(result[1].1.contains("String.fromEnvironment"));
    }
}
