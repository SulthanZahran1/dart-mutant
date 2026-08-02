# GOAL 2.0 — dart_mutant 1.1.0: Flutter per-test routing + incremental polish

> **Status:** draft (proposed 2026-08-02). Locking is a human act — see Human check.
> **Prerequisite:** GOAL-1.0.md signed-off (the harness exists and is green).

Make Flutter mutation runs fast enough for dev loops and sharpen the incremental story: per-test-file coverage routing for Flutter (research #6's 1.1.0 step), deeper incremental-mode behavior, and the first ecosystem integrations.

---

## Verifiable Acceptance Criteria

### 1. Flutter Per-Test Routing

`flutter test` runs per test file to recover per-test attribution (N compilation passes, slower than Dart's single-pass JSON, but correct), so a mutant is tested only against the test files that cover it.

**Test:** on the Flutter fixture with ≥5 test files, assert a mutant covered by 1 file executes only that file's tests.
**Pass:** routing correct; wall-clock on the Flutter fixture improves vs full-suite routing by ≥40% (or documented infeasible with numbers — see renegotiation).

### 2. Incremental-Mode Depth

`--incremental --base-ref` narrows to changed files **and** their direct dependents (import graph, not just changed files), with the content-addressed cache skipping byte-identical work.

**Test:** touch one file in `large/`; assert mutant count = changed + dependent files only, and warm rerun <15 s.
**Pass:** dependents included; warm rerun under the 30 s envelope.

### 3. Ecosystem Integrations

- **VS Code / CI badges:** a documented badge (or report artifact) that CI pipelines can post (e.g. MSI on the PR/commit status).
- **pre-commit / hook template:** a documented `dart_mutant --incremental` hook example.

**Test:** documented integration works from the docs' copy-paste commands on a fresh checkout.
**Pass:** commands run green in the human-check demo.

---

## Implementation Rules

- Same as GOAL-1.0.md (AGENTS.md, atomic commits, branch → PR → merge).
- Flutter routing work must not regress the Dart path — Dart integration suite stays green (GOAL-1.0 harness is the regression net).
- Performance claims measured on the committed fixtures, not ad-hoc projects.

---

## Human check

**Type:** Signed-off live demo (per protocol).

The agent runs, live:
1. Flutter fixture with routing on — show per-test execution counts and the speed delta vs full-suite.
2. Incremental demo on `large/` — touch a file, show the narrowed mutant set, warm timing.
3. Ecosystem integration demo — badge/hook from a fresh checkout.

**Sign-off / failure:** same mechanics as GOAL-1.0.md.
