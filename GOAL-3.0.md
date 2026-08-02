# GOAL 3.0 — dart_mutant 1.2.0: hybrid routing, advanced TCE, plugin surface

> **Status:** draft (proposed 2026-08-02). Locking is a human act — see Human check.
> **Prerequisite:** GOAL-2.0.md signed-off.

The final hardening milestone: hybrid routing (research #6's 1.2.0 step), stronger equivalent-mutant detection, and a stable surface for third-party operators.

---

## Verifiable Acceptance Criteria

### 1. Hybrid Routing

The runner chooses per-project: Dart projects use JSON per-test routing (single compile pass); Flutter projects use per-file routing; large Flutter projects can fall back to full-suite with a documented, measured trade-off. Routing strategy is a config option (`routing: auto|per-test|per-file|full-suite`).

**Test:** same fixture run under each routing strategy; assert identical classification outcomes (correctness invariant) and record timings.
**Pass:** outcomes identical across strategies; timings documented in the report's metadata.

### 2. Advanced TCE

Equivalent-mutant detection extends beyond bytecode comparison: e.g. dead-code elimination awareness, constant-folding equivalence classes, and a `--classify-equivalent` human-review aid that groups likely-equivalents with rationale.

**Test:** on a fixture seeded with N known-equivalent mutants, the tool flags ≥90% automatically (up from GOAL-1.0's ≥80%).
**Pass:** ≥90% flagged; false-positive rate <10% (human-checked sample).

### 3. Plugin Surface

A stable, documented API for third-party operators: operators are expressed as tree-sitter query patterns + replacements, loadable from a config (`operators: [{name, pattern, replace}]`), versioned against the frozen JSON schema.

**Test:** a custom operator defined purely in config mutates the expected nodes on the medium fixture.
**Pass:** documented config format works from scratch; schema validation green.

---

## Implementation Rules

- Same as previous milestones (AGENTS.md, atomic commits, branch → PR → merge).
- The plugin surface is additive to the frozen 1.0 contract — no breaking changes to flags, exit codes, or JSON fields.
- All prior milestone gates stay green (the harness is the regression net for everything).

---

## Human check

**Type:** Signed-off live demo (per protocol).

The agent runs, live:
1. Routing strategy matrix on one fixture — same outcomes, timings shown.
2. TCE seeded-equivalent demo — flag rate + false positives, human inspects a sample.
3. Custom operator defined live in config → mutants appear → report valid.

**Sign-off / failure:** same mechanics as GOAL-1.0.md.
