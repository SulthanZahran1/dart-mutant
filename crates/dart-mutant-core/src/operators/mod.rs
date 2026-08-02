//! Mutation operators.
//!
//! Each operator implements the [`Mutator`] trait, which scans Dart source
//! text and produces [`Mutant`](crate::Mutant) structs. The operators use
//! regex and line-based scanning (since `tree-sitter-dart` is not available
//! as a crate, we do line-based source mutation).
//!
//! ## Operator codes
//!
//! | Code    | Operator                          | Category    |
//! |---------|-----------------------------------|-------------|
//! | `AOR`   | Arithmetic Operator Replacement   | Generic     |
//! | `AOD`   | Arithmetic Operator Deletion       | Generic     |
//! | `AOI`   | Arithmetic Operator Insertion      | Generic     |
//! | `ROR`   | Relational Operator Replacement    | Generic     |
//! | `LOR`   | Logical Operator Replacement       | Generic     |
//! | `LCR`   | Logical Constant Replacement       | Generic     |
//! | `COR`   | Conditional Operator Replacement    | Generic     |
//! | `SDL`   | Statement Deletion                 | Generic     |
//! | `RVR`   | Return Value Replacement           | Generic     |
//! | `INC`   | Increment/Decrement swap           | Generic     |
//! | `NullSafety`  | `??` removal               | Dart-specific |
//! | `NullAssert`  | `!` removal                | Dart-specific |
//! | `OptionalChaining` | `?.` → `.`           | Dart-specific |
//! | `Cascade`     | `..` → `.`                  | Dart-specific |
//! | `AsyncAwait`  | remove `await`             | Dart-specific |
//! | `StreamMutation` | stream property swap    | Dart-specific |
//! | `SealedExhaustiveness` | remove switch case | Dart-specific |

pub mod arithmetic;
pub mod async_await;
pub mod cascade;
pub mod conditional;
pub mod logical;
pub mod loop_ops;
pub mod null_safety;
pub mod relational;
pub mod r#return;
pub mod sealed_class;
pub mod statement;
pub mod stream;

use crate::Mutant;

/// A mutation operator that finds mutation points in Dart source text.
///
/// Implementors scan source code line-by-line (or with regex) and return
/// [`Mutant`] structs describing each possible mutation. Each mutant carries
/// the `original` text and the `replacement` text so the schemata generator
/// can wrap it in a conditional branch.
///
/// The trait is `Send + Sync` so operators can be collected into a `Vec<Box<dyn Mutator>>`
/// and used from parallel code.
pub trait Mutator: Send + Sync {
    /// Human-readable name of the operator (e.g. `"Arithmetic Operator Replacement"`).
    fn name(&self) -> &str;

    /// Short operator code (e.g. `"AOR"`).
    fn code(&self) -> &str;

    /// Scan `source` (the full file text) and return all mutants found in it.
    ///
    /// `file_path` is the path to the source file, used to populate
    /// [`Mutant::file_path`].
    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant>;
}

/// Returns a list of all built-in mutator instances.
///
/// This is the standard operator set used by the orchestrator. Each entry
/// is a boxed `dyn Mutator`.
pub fn all_operators() -> Vec<Box<dyn Mutator>> {
    vec![
        Box::new(arithmetic::ArithmeticOperatorReplacement),
        Box::new(arithmetic::ArithmeticOperatorDeletion),
        Box::new(arithmetic::ArithmeticOperatorInsertion),
        Box::new(relational::RelationalOperatorReplacement),
        Box::new(logical::LogicalOperatorReplacement),
        Box::new(logical::LogicalConstantReplacement),
        Box::new(conditional::ConditionalOperatorReplacement),
        Box::new(statement::StatementDeletion),
        Box::new(r#return::ReturnValueReplacement),
        Box::new(loop_ops::IncrementDecrement),
        Box::new(null_safety::NullSafetyOperator),
        Box::new(null_safety::NullAssertOperator),
        Box::new(null_safety::OptionalChainingOperator),
        Box::new(cascade::CascadeOperator),
        Box::new(async_await::AsyncAwaitOperator),
        Box::new(stream::StreamMutationOperator),
        Box::new(sealed_class::SealedExhaustivenessOperator),
    ]
}

/// Returns the list of Dart-specific operator codes.
pub fn dart_specific_codes() -> &'static [&'static str] {
    &[
        "NullSafety",
        "NullAssert",
        "OptionalChaining",
        "Cascade",
        "AsyncAwait",
        "StreamMutation",
        "SealedExhaustiveness",
    ]
}

/// Returns the list of generic (language-agnostic) operator codes.
pub fn generic_codes() -> &'static [&'static str] {
    &[
        "AOR", "AOD", "AOI", "ROR", "LOR", "LCR", "COR", "SDL", "RVR", "INC",
    ]
}

// ---------------------------------------------------------------------------
// Shared helpers used by all line-scanning operators
// ---------------------------------------------------------------------------

/// Find the byte offset of the start of a given 1-based line in `source`.
/// Returns `None` if the line number is out of range.
#[allow(dead_code)]
pub(crate) fn line_start_byte(source: &str, line: usize) -> Option<usize> {
    if line == 0 {
        return None;
    }
    let mut current_line = 1usize;
    let mut byte_idx = 0usize;
    for (i, ch) in source.char_indices() {
        if current_line == line {
            return Some(i);
        }
        if ch == '\n' {
            current_line += 1;
        }
        byte_idx = i;
    }
    // Handle last line if it doesn't end with \n
    if current_line == line {
        // byte_idx is at last char; return end
        return Some(source.len().min(byte_idx + 1));
    }
    let _ = byte_idx;
    None
}

/// Convert a byte offset in `source` to a (line, column) pair (both 1-based).
#[allow(dead_code)]
pub(crate) fn byte_to_line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Check if a byte offset is inside a string literal or comment on the given line.
///
/// This is a heuristic: we look at the line text and check if the column is
/// between matching quote characters or after `//`.
pub(crate) fn is_in_string_or_comment(line_text: &str, col: usize) -> bool {
    let bytes = line_text.as_bytes();
    if col == 0 || col > bytes.len() {
        return false;
    }
    let before = &line_text[..col.saturating_sub(1).min(bytes.len())];
    // Check for // comment
    if before.contains("//") {
        let comment_pos = before.find("//").unwrap();
        if col > comment_pos {
            return true;
        }
    }
    // Check for string literals: count unescaped quotes
    let mut in_string = false;
    let mut quote_char = b'"';
    let mut escaped = false;
    for &b in before.as_bytes() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            continue;
        }
        if in_string {
            if b == quote_char {
                in_string = false;
            }
        } else if b == b'"' || b == b'\'' {
            in_string = true;
            quote_char = b;
        }
    }
    in_string
}

/// Strip trailing comment from a line (for scanning purposes).
pub(crate) fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(pos) => &line[..pos],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_line_col() {
        let src = "abc\ndef\nghi";
        assert_eq!(byte_to_line_col(src, 0), (1, 1));
        assert_eq!(byte_to_line_col(src, 4), (2, 1));
        assert_eq!(byte_to_line_col(src, 5), (2, 2));
        assert_eq!(byte_to_line_col(src, 8), (3, 1));
    }

    #[test]
    fn test_line_start_byte() {
        let src = "abc\ndef\nghi";
        assert_eq!(line_start_byte(src, 1), Some(0));
        assert_eq!(line_start_byte(src, 2), Some(4));
        assert_eq!(line_start_byte(src, 3), Some(8));
        assert_eq!(line_start_byte(src, 4), None);
    }

    #[test]
    fn test_all_operators_count() {
        let ops = all_operators();
        // 10 generic + 7 dart-specific = 17
        assert_eq!(ops.len(), 17);
    }

    #[test]
    fn test_is_in_string_or_comment() {
        assert!(is_in_string_or_comment("var x = \"hello + world\";", 16));
        assert!(!is_in_string_or_comment("var x = a + b;", 10));
        assert!(is_in_string_or_comment("var x = 1; // comment", 18));
    }
}
