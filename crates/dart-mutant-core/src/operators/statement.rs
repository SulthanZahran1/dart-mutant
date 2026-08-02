//! SDL — Statement Deletion.
//!
//! Removes entire statements from the source. This tests whether each
//! statement's side effects are verified by the test suite.
//!
//! The operator scans for statements that end with `;` and removes them
//! (replaces with an empty statement or comment). It avoids removing
//! control-flow keywords (`if`, `for`, `while`, `return`, etc.) and only
//! targets executable statements.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// SDL — Statement Deletion.
///
/// Removes statements (lines ending with `;` that are not declarations
/// without initialization or control-flow constructs).
pub struct StatementDeletion;

impl Mutator for StatementDeletion {
    fn name(&self) -> &str {
        "Statement Deletion"
    }

    fn code(&self) -> &str {
        "SDL"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            let trimmed = clean.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }
            // Skip lines that are just braces
            if trimmed == "{" || trimmed == "}" || trimmed == "};" {
                continue;
            }
            // Skip control-flow statements (if, else, for, while, do, switch, case, etc.)
            if is_control_flow_keyword(trimmed) {
                continue;
            }
            // Skip function/class/typedef declarations
            if is_declaration(trimmed) {
                continue;
            }
            // Skip import/export/library/part directives
            if is_directive(trimmed) {
                continue;
            }
            // Skip lines that don't end with `;` or look like a statement
            if !trimmed.ends_with(';') {
                continue;
            }
            // Skip lines that start with `//` (already stripped, but double-check)
            if trimmed.starts_with("//") {
                continue;
            }
            // Check for string/comment at the start
            if is_in_string_or_comment(line, 1) {
                continue;
            }

            // Only mutate if the line looks like a simple statement
            // (not a multi-line construct starting on a previous line).
            // We check that it's not a continuation (doesn't start with an operator or comma).
            if trimmed.starts_with(',')
                || trimmed.starts_with('+')
                || trimmed.starts_with('-')
                || trimmed.starts_with('*')
                || trimmed.starts_with('/')
                || trimmed.starts_with('|')
                || trimmed.starts_with('&')
                || trimmed.starts_with('?')
            {
                continue;
            }

            let original = trimmed.to_string();
            // Replace the statement with an empty comment (to preserve line structure)
            let replacement = "/* SDL: statement removed */".to_string();
            mutants.push(Mutant::without_id(
                file_path,
                line_no,
                1,
                "SDL",
                &original,
                &replacement,
                format!("SDL: remove statement at line {}", line_no),
            ));
        }

        mutants
    }
}

fn is_control_flow_keyword(trimmed: &str) -> bool {
    let lower = trimmed.trim_start();
    // Check if line starts with a control-flow keyword followed by space or `(`.
    let keywords = [
        "if ",
        "if(",
        "else ",
        "else{",
        "else}",
        "for ",
        "for(",
        "while ",
        "while(",
        "do ",
        "do{",
        "switch ",
        "switch(",
        "case ",
        "default:",
        "default ",
        "break;",
        "continue;",
        "return ",
        "return;",
        "try ",
        "try{",
        "catch ",
        "catch(",
        "finally ",
        "finally{",
        "throw ",
        "rethrow;",
        "assert ",
        "assert(",
    ];
    for kw in &keywords {
        if lower.starts_with(kw) {
            return true;
        }
    }
    false
}

fn is_declaration(trimmed: &str) -> bool {
    // Function/method declarations: contain `(` and `)` and `=>` or `{`
    // Class/enum/mixin/extension/typedef declarations
    let decl_keywords = [
        "class ",
        "enum ",
        "mixin ",
        "extension ",
        "typedef ",
        "abstract ",
    ];
    for kw in &decl_keywords {
        if trimmed.starts_with(kw) {
            return true;
        }
    }
    // Skip method signatures that look like declarations (contain `(` and `)` and `{`)
    // But allow variable declarations (which also end with `;`)
    // We only skip if it's a class/enum/etc. declaration
    false
}

fn is_directive(trimmed: &str) -> bool {
    let directives = [
        "import ", "export ", "library ", "part ", "@", "import'", "import\"", "export'",
        "export\"",
    ];
    for d in &directives {
        if trimmed.starts_with(d) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdl_finds_assignment() {
        let src = "void main() {\n  var x = 1;\n  print(x);\n}\n";
        let m = StatementDeletion;
        let mutants = m.find_mutations(src, "test.dart");
        // `var x = 1;` and `print(x);` should be found
        assert!(!mutants.is_empty());
        assert!(mutants.iter().all(|m| m.operator == "SDL"));
    }

    #[test]
    fn test_sdl_skips_control_flow() {
        let src = "if (x > 0) return x;\n";
        let m = StatementDeletion;
        let mutants = m.find_mutations(src, "test.dart");
        // `if (...) return x;` starts with `if ` — should be skipped
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_sdl_skips_imports() {
        let src = "import 'dart:io';\n";
        let m = StatementDeletion;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_sdl_skips_braces() {
        let src = "}\n{\n};\n";
        let m = StatementDeletion;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }
}
