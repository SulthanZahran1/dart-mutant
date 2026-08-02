# GOAL — dart_mutant

> **Status:** live spec — in flight, not yet implemented.

Build an AST-based mutation testing tool for Dart that compiles once via mutant schemata, routes each mutant only to covering tests, classifies all 6 mutant states including equivalent-mutant detection via Dart kernel/bytecode comparison, produces Stryker-compatible JSON + JUnit XML + HTML reports, implements ≥6 Dart-specific operators (null safety, cascade, async/await, streams, sealed classes), and completes a 500-mutant cold run in under 10 minutes with a <2% compilation failure rate.

---

## Verifiable Acceptance Criteria

Each criterion below is measurable. Build a test suite that validates each one.

### 1. AST-Based Mutations (Not Regex/Text)

The tool parses Dart source code into an Abstract Syntax Tree (tree-sitter or Dart analyzer) and mutates tree nodes. Every mutation produces syntactically valid Dart code that compiles.

**Test:** Generate 1,000 mutants from a real Dart codebase. Count how many fail to compile.
**Pass:** <2% compilation failure rate.

### 2. Language-Specific Mutation Operators

The tool implements operators that target Dart's unique constructs, not just generic arithmetic/logic swaps.

**Minimum generic operator set (verifiable count):**

| Category | Operators | Example |
|---|---|---|
| Arithmetic | AOR, AOD, AOI | `+` → `-`, `*` → `/` |
| Relational | ROR | `>` → `>=`, `==` → `!=` |
| Logical | LOR, LCR | `&&` → `\|\|`, `true` → `false` |
| Conditional | COR, negate | `if (x)` → `if (!x)` |
| Statement | SDL | Remove statement |
| Return | RVR | Replace return value with zero/empty |
| Loop | inc/dec | `i++` → `i--` |

**Dart-specific operators (the differentiator — ≥6 required):**

| Operator | What it does |
|---|---|
| NullSafety (`??` → remove) | Tests catch missing null fallbacks |
| NullAssert (`!` → remove) | Tests catch unsafe null force-unwraps |
| OptionalChaining (`?.` → `.`) | Tests catch null propagation gaps |
| Cascade (`..` → `.`) | Dart-unique — tests catch cascade misuse |
| AsyncAwait (remove `await`) | Tests catch missing async synchronization |
| StreamMutation | Swap stream operations |
| SealedClassExhaustiveness | Remove a branch from `switch` on sealed type |

**Test:** Count language-specific operators. Count generic operators.
**Pass:** ≥6 Dart-specific operators that no generic tool could produce. ≥7 generic operators.

### 3. Performance — Mutant Schemata + Coverage Routing

The tool does NOT recompile per-mutant. It uses **mutant schemata** (inject all mutations into a single compilation, switch at runtime via env var/flag) AND **per-test coverage routing** (only run tests whose coverage touches the mutated line).

**Metrics:**

| Technique | Effect | Verifiable by |
|---|---|---|
| Mutant schemata | Compile once, not N times | Measure: compilation calls = 1, not N |
| Per-test coverage routing | Skip tests that don't cover mutated line | Measure: avg tests run per mutant < total test count |
| Parallel execution | Utilize all cores | Measure: wall-clock scales linearly with `--jobs` |
| Incremental mode | Only mutate changed lines (`--since git_ref`) | Measure: mutant count on unchanged code = 0 |
| Caching | Skip mutants whose source + tests are byte-identical to previous run | Measure: warm rerun < 5% of cold run time |

**Test:** Run on a 50-file project with 500 mutants.
**Pass:** Cold run < 10 minutes. Warm rerun < 30 seconds.

### 4. Equivalent Mutant Detection

The tool classifies mutants that produce identical behavior as `EQUIVALENT` and excludes them from the mutation score denominator.

**Approach:** Recompile survivor, compare Dart kernel/bytecode (via `dart compile kernel` or `dart compile js`). If identical → equivalent. This is Trivial Compiler Equivalence (TCE).

**Test:** Run on a codebase. Count survivors manually classified as equivalent by a human.
**Pass:** Tool flags ≥80% of equivalent mutants automatically.

### 5. Reporting & CI Integration

The tool produces machine-readable output that integrates with existing CI dashboards.

**Required output formats:**

| Format | Purpose | Verifiable |
|---|---|---|
| Stryker-compatible JSON | mutation-testing-elements dashboard | Validate against Stryker JSON schema |
| JUnit XML | CI test result panels (GitHub, GitLab, Azure) | `xmllint --schema junit.xsd` passes |
| HTML report | Human review, self-contained | Opens in browser, shows per-file mutation map |
| Console summary | Quick terminal feedback | Shows MSI, killed/survived/timeout counts |
| Exit codes | CI gate (0=pass, 1=below threshold) | `echo $?` returns correct code |

**Test:** Generate all 5 formats from a single run. Validate each against its schema.
**Pass:** 5/5 valid.

### 6. Timeout Safety & Mutant Classification

Every mutant gets classified into exactly one of:

| Status | Meaning |
|---|---|
| **KILLED** | Test suite failed → mutation detected |
| **SURVIVED** | Test suite passed → mutation undetected |
| **TIMEOUT** | Mutation caused infinite loop/hang → detected via timeout |
| **EQUIVALENT** | Mutation produces identical behavior → unkillable |
| **NOT_COVERED** | No test covers the mutated line → skip |
| **COMPILE_ERROR** | Mutation produces invalid code → skip (should be <2%) |

**Timeout mechanism:** Adaptive per-mutant timeout based on baseline test duration × multiplier (not a fixed global timeout).

**Test:** Introduce an infinite-loop mutant (`i++` → `i--` in a for loop).
**Pass:** Classified as TIMEOUT within `baseline × 3` seconds, not hung indefinitely.
