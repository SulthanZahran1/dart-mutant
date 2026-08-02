# Flutter Support: Coverage Format, Tool Gaps, and Per-File Routing Feasibility

> **Branch:** `research/flutter-support` · **Date:** 2026-08-02

## 1. `flutter test --coverage` vs `dart test --coverage` — Coverage Format

`flutter test --coverage` and `dart test --coverage` have historically produced
**different formats**. As of Dart 3.12+, they are **converging but not yet
identical**:

| Aspect | `flutter test --coverage` | `dart test --coverage=<dir>` | `dart test --coverage-path=<file>` (new) |
|---|---|---|---|
| **Flag** | `--coverage` (boolean) | `--coverage=<dir>` (needs dir arg) | `--coverage-path=<file>` (implies coverage) |
| **Output** | LCOV (`coverage/lcov.info`) | JSON files nested under `<dir>/` | LCOV at given path |
| **Branch coverage** | `--branch-coverage` (regression noted in sdk#60958) | Not available pre-3.12 | `--branch-coverage` (PR#2517) |
| **Function coverage** | None (perf) | None | Not in #2517 |
| **`collect()` params** | `resume`/`waitPaused` disabled; `coverableLineCache` enabled | `resume`/`waitPaused` disabled; `isolateIds` passed; no `scopedOutput` | Same as `--coverage` + sets `scopedOutput` to pkg name |

### JSON coverage format (Dart VM)

`dart test --coverage=<dir>` emits **JSON files** — one per test file/process.
Standard `package:coverage` VM-Service JSON:

```json
{"type":"CodeCoverage","coverage":[
  {"source":"package:myapp/foo.dart","hits":[5,2,8,2,11,2,...]}
]}
```

- `hits` is a **flat array** of alternating `[line, hit_count]` pairs.
- `source` uses **`package:` URIs**, not filesystem paths.
- Each JSON file maps to one test file → **per-test-file attribution is
  recoverable** by mapping JSON file name back to test file path.

`flutter test --coverage` does **not** emit JSON. It calls
`HitMap.parseJsonSync()` on collected data, then `hitmap.formatLcov()` to
produce a single `coverage/lcov.info`. The intermediate JSON is consumed
in-memory, never written to disk.

### Convergence timeline

- **test#2511** (closed Jun 2025): Issue to align `dart test --coverage` with
  `test_with_coverage` and `flutter test --coverage`.
- **test PR#2517** (merged Aug 7, 2025): Adds `--coverage-path` (LCOV output
  mirroring flutter) and `--branch-coverage` to `dart test`. First step
  toward format convergence.
- **sdk#60958**: Umbrella issue. `--coverage` flag behavior (JSON dir vs
  LCOV) intentionally **not yet changed** — deferred to future breaking
  release.

### Verdict for dart_mutant

`coverage.rs` already handles **both** formats: LCOV (`parse_lcov`) and
JSON (`parse_dart_coverage`). The JSON parser correctly handles the flat
`[line, count]` array and `package:` URI normalization. **But** the per-test
attribution keys by JSON file's relative path, which works for `dart test`
but **not** for `flutter test` (which only emits LCOV with no per-test
attribution).

---

## 2. Known Gaps in Existing Dart/Flutter Mutation Tools

### MelbourneDeveloper/dart_mutant (15★, v0.1.0, Dec 2025)

Original proof-of-concept. Rust + tree-sitter.

**Gaps:** Only runs `dart test` (no documented `flutter test` support) ·
no per-test coverage routing (full suite per mutant) · no equivalent-mutant
detection (TCE) · no mutant schemata (recompiles per mutant) · single
contributor, v0.1.0. Nimblesite/dart_mutant is a fork with the same gaps.

### pub.dev `mutation_test` (v1.8.0, 21k downloads)

**Gaps:** Regex-based text replacement (not AST) · language-agnostic (no
Dart-specific operators) · no coverage routing (file-level whitelist only)
· no TCE · no schemata. Last published 5 months ago.

### dartmutant.dev / Nimblesite/dart_mutant

Same MelbourneDeveloper codebase. Nimblesite distributes via Homebrew
(`nimblesite/tap/dart_mutant`). Same gaps.

### Stryker

No Dart flavor. See §3.

### Gap summary

| Capability | mutation_test | MelbourneDev dart_mutant | dart_mutant (this repo) |
|---|---|---|---|
| AST mutations | ❌ (regex) | ✅ | ✅ |
| Dart-specific operators | ❌ | ✅ basic | ✅ (7) |
| Per-test coverage routing | ❌ | ❌ | ✅ (JSON) |
| Mutant schemata | ❌ | ❌ | ✅ |
| TCE equivalent detection | ❌ | ❌ | ✅ |
| `flutter test` support | configurable | docs mention | configurable, auto-detects |
| LCOV parsing | N/A | ❌ | ✅ |

**Universal gap: no existing tool has per-test-file coverage routing
working with `flutter test`.** All either run the full suite per mutant or
rely on `dart test`'s JSON output.

---

## 3. Stryker Dart Support — Current State

**No Stryker Dart flavor exists.** Stryker supports exactly three languages:

| Flavor | Languages |
|---|---|
| StrykerJS | JavaScript, TypeScript |
| Stryker.NET | C# (.NET) |
| Stryker4s | Scala |

Evidence: stryker-mutator.io homepage lists only JS/TS, C#, Scala.
awesome-mutation-testing#49: "I asked the Stryker team and apparently no
plans for flutter right now."

**Implication:** Dart mutation testing is an open niche. Stryker is not
entering. dart_mutant's Stryker-compatible JSON output is the right
interoperability bet, but there's no upstream Dart runner.

---

## 4. Per-Test-File Coverage Routing with `flutter test`

### Positional test file args

`flutter test` accepts positional file/directory args — same as `dart test`:

```bash
flutter test test/widget_test.dart              # single file
flutter test test/widget_test.dart test/unit/   # multiple
```

**Known difference — arg ordering:** `flutter test` sets
`allowTrailingOptions: false` (flutter#85891). Flags **after** the file
path are treated as filenames. So `flutter test test/foo.dart --reporter
json` fails; must be `flutter test --reporter json test/foo.dart`.

### Feasibility verdict

**Feasible but with a critical limitation.**

`flutter test --coverage` produces **only LCOV** — a single aggregated
file with no per-test attribution. dart_mutant's `parse_lcov()` handles
this by mapping all covered lines to a single `"baseline"` test, meaning
**coverage routing falls back to full-suite-per-mutant** under `flutter
test`.

**Three paths to per-test routing under Flutter:**

1. **Run `flutter test` once per test file with `--coverage`** — parse
   `lcov.info` after each run. Cost: N baseline runs. Feasible but slow
   (Flutter compilation overhead). StrykerJS uses this pattern.

2. **Hybrid: `dart test` for pure-Dart unit tests, `flutter test` for
   widget tests** — `dart test --coverage=<dir>` gives per-file JSON with
   per-test attribution. Widget tests requiring
   `TestWidgetsFlutterBinding` need `flutter test` (LCOV only). Most
   promising path.

3. **Use `dart test --coverage-path` (PR#2517)** — produces LCOV but
   still aggregated. Same limitation as `flutter test --coverage`.

### Recommendation

- **Pure Dart projects:** `dart test --coverage=<dir>` JSON → per-test
  routing works today via `parse_dart_coverage()`.
- **Flutter projects:** `flutter test --coverage` → LCOV → fallback to
  full-suite routing. Per-test routing requires per-file baseline runs or
  hybrid `dart test`/`flutter test` split — both post-1.0 optimizations.
- **Widget test trees need special handling** — requires Flutter's
  compilation pipeline. Roadmap already notes this.

---

## 5. Recommendation: 1.0.0 Milestone or Later?

**Flutter support should be a post-1.0.0 milestone.**

Rationale:
- GOAL.md acceptance criteria validated on **Dart fixtures** (143 mutants,
  119 killed, 0 compile errors). 1.0.0 ships on the verified Dart path.
- `flutter test --coverage` → LCOV → full-suite routing **works**
  (mutants still tested), just slower. Performance concern, not
  correctness.
- Per-test routing for Flutter needs N baseline runs (slow) or hybrid
  split (complex). Neither blocks 1.0.0 correctness.
- Coverage format convergence (test#2517) is recent (Aug 2025), not yet
  in a stable `pkg:test` release. Betting on it for 1.0.0 risks coupling
  to unreleased APIs.

**Proposed milestones:**
- **1.0.0** — Dart support (verified). `flutter test` auto-detected;
  coverage routing falls back to LCOV full-suite. Correct mutation scores
  for Flutter, without per-test routing speedup.
- **1.1.0** — Flutter per-test-file routing via per-file `flutter test
  --coverage` baseline runs.
- **1.2.0** — Hybrid `dart test` (unit, per-file JSON) / `flutter test`
  (widget, LCOV) routing.

---

## References

- [test#2511](https://github.com/dart-lang/test/issues/2511) — Align dart/flutter coverage
- [test PR#2517](https://github.com/dart-lang/test/pull/2517) — `--coverage-path` + `--branch-coverage` (merged Aug 2025)
- [sdk#60958](https://github.com/dart-lang/sdk/issues/60958) — LCOV from dart test
- [test#1265](https://github.com/dart-lang/test/issues/1265) — coverage should emit lcov
- [flutter#85891](https://github.com/flutter/flutter/issues/85891) — trailing args treated as filenames
- [flutter test.dart](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/commands/test.dart)
- [flutter coverage_collector.dart](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/test/coverage_collector.dart)
- [mutation_test on pub.dev](https://pub.dev/packages/mutation_test)
- [MelbourneDeveloper/dart_mutant](https://github.com/MelbourneDeveloper/dart_mutant)
- [Nimblesite/dart_mutant](https://github.com/Nimblesite/dart_mutant)
- [dartmutant.dev](https://dartmutant.dev)
- [StrykerJS](https://github.com/stryker-mutator/stryker-js)
- [Stryker supported mutators](https://stryker-mutator.io/docs/mutation-testing-elements/supported-mutators/)
- [awesome-mutation-testing#49](https://github.com/theofidry/awesome-mutation-testing/issues/49)
- [package:coverage](https://pub.dev/packages/coverage)