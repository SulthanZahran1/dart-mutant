//! Async/await operator: remove `await`.
//!
//! Removing `await` from `await f()` changes a Future's resolution — the
//! code continues without waiting for the async operation to complete.
//! This tests whether the suite catches missing async synchronization.

use crate::operators::{is_in_string_or_comment, strip_comment, Mutator};
use crate::Mutant;
use regex::Regex;

/// AsyncAwait — Remove `await` keyword.
///
/// `await f()` → `f()` (removes the async synchronization point).
pub struct AsyncAwaitOperator;

impl Mutator for AsyncAwaitOperator {
    fn name(&self) -> &str {
        "AsyncAwait"
    }

    fn code(&self) -> &str {
        "AsyncAwait"
    }

    fn find_mutations(&self, source: &str, file_path: &str) -> Vec<Mutant> {
        let mut mutants = Vec::new();

        // Match `await` as a standalone keyword followed by an expression.
        // We need word-boundary matching: `await ` or `await(`.
        let re = Regex::new(r"\bawait\b\s*").unwrap();

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

                // The matched text includes trailing whitespace. Get the original `await` text.
                let matched = m.as_str();
                // The original is `await ` (with trailing space) or `await`
                // The replacement is empty string (remove await entirely)
                // But we need to be careful: the replacement should not leave
                // a dangling expression. So we replace `await ` with `` (empty).
                let original = matched.to_string();
                let replacement = String::new(); // Remove `await ` entirely

                mutants.push(Mutant::without_id(
                    file_path,
                    line_no,
                    col,
                    "AsyncAwait",
                    &original,
                    &replacement,
                    format!("AsyncAwait: remove `await` at line {}", line_no),
                ));
            }
        }

        mutants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_await_removes_keyword() {
        let src = "var x = await fetchData();\n";
        let m = AsyncAwaitOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 1);
        assert!(mutants[0].original.contains("await"));
        assert!(mutants[0].replacement.is_empty());
    }

    #[test]
    fn test_async_await_skips_strings() {
        let src = "var s = \"await future\";\n";
        let m = AsyncAwaitOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert!(mutants.is_empty());
    }

    #[test]
    fn test_async_await_multiple() {
        let src = "var a = await f1();\nvar b = await f2();\n";
        let m = AsyncAwaitOperator;
        let mutants = m.find_mutations(src, "test.dart");
        assert_eq!(mutants.len(), 2);
    }

    #[test]
    fn test_async_await_not_in_identifier() {
        let src = "var awaitResult = 1;\n";
        let m = AsyncAwaitOperator;
        let mutants = m.find_mutations(src, "test.dart");
        // `awaitResult` should not match (word boundary)
        assert!(mutants.is_empty());
    }
}
