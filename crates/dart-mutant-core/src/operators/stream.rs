//! Stream mutations: swap stream properties and operations.
//!
//! Streams are core to Dart/Flutter state management. This operator swaps
//! common stream properties and methods to test whether the suite catches
//! wrong stream consumption.
//!
//! Mutations:
//! - `.first` → `.last`
//! - `.last` → `.first`
//! - `.isEmpty` → `.isNotEmpty`
//! - `.isNotEmpty` → `.isEmpty`
//! - `.length` → `0` (replace with constant)

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;

/// StreamMutation — Swap stream properties.
pub struct StreamMutationOperator;

impl Mutator for StreamMutationOperator {
    fn name(&self) -> &str {
        "StreamMutation"
    }

    fn code(&self) -> &str {
        "StreamMutation"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        let swaps: &[(&str, &str)] = &[
            (".first", ".last"),
            (".last", ".first"),
            (".isEmpty", ".isNotEmpty"),
            (".isNotEmpty", ".isEmpty"),
            (".toList()", ".toSet()"),
            (".toSet()", ".toList()"),
        ];

        for (line_num, line) in source.lines().enumerate() {
            let line_no = line_num + 1;
            let clean = strip_comment(line);
            if clean.trim().is_empty() {
                continue;
            }

            for &(from, to) in swaps {
                let mut start = 0;
                while let Some(pos) = clean[start..].find(from) {
                    let abs_pos = start + pos;
                    let col = abs_pos + 1;
                    if is_in_string_or_comment(line, col) {
                        start = abs_pos + from.len();
                        continue;
                    }
                    mutants.push(Mutant::without_id(
                        file_path,
                        line_no,
                        col,
                        "StreamMutation",
                        from,
                        to,
                        format!("StreamMutation: {} → {} at line {}", from, to, line_no),
                    ));
                    start = abs_pos + from.len();
                }
            }
        }

        mutants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_first_to_last() {
        let src = "var x = stream.first;\n";
        let m = StreamMutationOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, ".first");
        assert_eq!(mutants[0].replacement, ".last");
    }

    #[test]
    fn test_stream_isempty_to_isnotempty() {
        let src = "if (list.isEmpty) return;\n";
        let m = StreamMutationOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, ".isEmpty");
        assert_eq!(mutants[0].replacement, ".isNotEmpty");
    }

    #[test]
    fn test_stream_tolist_toset() {
        let src = "var items = stream.toList();\n";
        let m = StreamMutationOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].original, ".toList()");
        assert_eq!(mutants[0].replacement, ".toSet()");
    }

    #[test]
    fn test_stream_skips_strings() {
        let src = "var s = \".first\";\n";
        let m = StreamMutationOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }
}
