# GOAL 1.0.0 — dart_mutant: the verified harness

> **Status:** draft (proposed 2026-08-02). Locking is a human act — see Human check.
> **Scope decided in wayfinder ticket #2:** harness hard requirement · Flutter IN · Windows IN · scale gate IN · crates.io IN · CLI/JSON contract frozen.

Build the integration test harness and release machinery that make dart_mutant's guarantees machine-checked, so that 1.0.0 is a version whose claims are proven in CI on every commit: an integration test crate with small/medium/large + Flutter fixtures, a CI matrix on Linux and Windows against real Dart and Flutter SDKs, a performance scale gate, a frozen CLI/JSON contract with `schemaVersion`, and publishing to crates.io on tag.

---

## Verifiable Acceptance Criteria

Each criterion is measurable and machine-checkable in CI or the human-check demo.

### 1. Integration Test Harness

The repo gains a `dart-mutant-tests` integration crate (or equivalent `tests/` layout) with real Dart fixtures committed to the repo, and integration tests that run the actual CLI binary against them.

**Fixtures (per research #5 sizing):**
- `small/` — 1 file, ~12 mutants, 100% kill, <5 s
- `medium/` — 3–4 files, 50–80 mutants, mixed outcomes (killed + survived + timeout), <30 s
- `large/` — 8–10 files, 200+ mutants, realistic MSI (~65%), <5 min
- `flutter/` — a minimal Flutter app (widget tests) exercising `flutter test` execution path

**Test:** `cargo test` in CI runs the integration suite against all fixtures.
**Pass:** All integration tests pass; fixture set matches the above sizes.

### 2. CI Matrix — Linux + Windows, Real SDKs

Integration tests run in GitHub Actions on `ubuntu-latest` **and** `windows-latest`, against the **real Dart SDK** (`dart-lang/setup-dart`) and the **real Flutter SDK** (flutter action) — never a mock.

**Test:** CI job matrix `{ubuntu-latest, windows-latest}` × `{dart, flutter}`.
**Pass:** All four combinations green on every push to `main`.

### 3. Correctness Gates (ported from GOAL-0.1.md, now machine-checked)

- **Compile-error rate:** <2% of mutants across the medium + large fixtures (research #5: PIT's bar is ~0 — treat <2% as a floor, not a goal).
- **Coverage routing:** a mutant covered by 1 of N tests only executes that 1 test (assert via test-count instrumentation).
- **Schemata:** compilation happens once, not N times (assert via instrumentation or timing).
- **Classification:** all 6 statuses (KILLED/SURVIVED/TIMEOUT/EQUIVALENT/NOT_COVERED/COMPILE_ERROR) appear correctly across fixtures; timeout via adaptive per-mutant timeout.
- **TCE:** ≥80% of human-classified equivalent mutants are flagged automatically.
- **Reports:** Stryker JSON validates against the mutation-testing-elements schema; JUnit XML passes `xmllint --schema junit.xsd`; HTML report opens and renders.

**Test:** integration suite assertions per gate.
**Pass:** all gates green in CI.

### 4. Performance Scale Gate

The `large/` fixture must meet GOAL-0.1.md criterion 3's performance envelope, now machine-measured in CI.

**Test:** `dart_mutant --path tests/fixtures/large` cold run; rerun immediately after for warm.
**Pass:** cold <10 min; warm <30 s (content-addressed cache hit).

### 5. Flutter Execution Path

`dart_mutant` runs against a Flutter project: discovers `flutter test` as the test command, parses LCOV coverage (`coverage/lcov.info`), and produces correct results via full-suite routing (per research #6 — per-test routing is explicitly post-1.0.0).

**Test:** `dart_mutant --path tests/fixtures/flutter` produces a valid report with all mutants classified.
**Pass:** report is valid; every mutant classified; MSI matches a reference value computed by hand for the fixture.

### 6. CLI/JSON Contract Freeze + schemaVersion

The machine contract is frozen and versioned:

- **Flags:** the documented flag set (including `--path`, `--threshold`, `--format`, `--quiet`, `--no-color`, `--mutant <id>`, `--parallel`, `--sample`, `--incremental --base-ref`, `--detect-equivalent`, `--exclude`) is frozen for 1.0.x.
- **Exit codes:** 0 = pass, 1 = below threshold, 2 = error. Frozen.
- **JSON:** output gains a `schemaVersion` field (e.g. `"schemaVersion": "1.0"`). Fields `mutationScore`, `killed`, `survived`, `timeout`, `equivalent`, `notCovered`, `compileError`, `files[]` are frozen for 1.0.x. Additive changes only.
- **Breaking changes** post-1.0.0 require a major version bump.

**Test:** integration test asserts `schemaVersion` present and parses JSON against a committed schema fixture; a golden-file test pins the JSON shape.
**Pass:** golden test green; schema fixture validated.

### 7. Publishing: crates.io

`cargo publish` is wired into the release workflow: tagging `v1.0.0` publishes the crate to crates.io, and the README's `cargo install dart-mutant` claim becomes true and is verified.

**Test:** after `v1.0.0` tag: `cargo install dart-mutant` on a clean machine exits 0; `dart_mutant --version` prints 1.0.0.
**Pass:** install E2E green (recorded in the human-check demo).

---

## Implementation Rules

- Follow AGENTS.md (AST-only mutations, schemata, atomic commits, branch → PR → merge).
- The integration crate is the primary consumer of the CLI as a subprocess — never test internals only; the binary's contract is what's frozen.
- Golden files (JSON shape, report snapshots) are committed and updated only with review — they are the freeze.
- Commit each piece as soon as it's verified (`cargo test` + `cargo clippy` before commit).
- No direct pushes to `main`.

---

## Human check

**Type:** Signed-off live demo (per wayfinder ticket #3 protocol).

The agent runs, live, in front of the human:
1. The full integration suite on Linux (all fixtures, all gates).
2. The Windows CI job results (or a live run if the machine allows).
3. The Flutter fixture run — real report, real classification.
4. The performance gate: cold + warm timing on `large/`.
5. `cargo install dart-mutant` from crates.io on a clean machine → `dart_mutant --version` = 1.0.0.
6. A real-project run chosen by the human (any Dart/Flutter project) — the human judges fit.

**Sign-off:** the human signs off in-session; the agent records it here and flips status to `signed-off`.

**Failure:** small failures → rework (criteria do not bend). Big failures (premise doesn't hold — e.g. Flutter path unusable in practice, performance envelope infeasible) → renegotiate criteria, then re-demo.
