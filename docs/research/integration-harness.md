# Integration/E2E Test Harness Patterns for Mutation Testing Tools

> Research notes for **dart-mutant**'s `dart-mutant-tests` crate. Examines how
> mature mutation testing tools structure their integration/E2E fixtures,
> what they assert, and whether they use real SDKs in CI. Based on docs and
> source for PIT, Stryker, go-mutesting, mutahunter, domohuhn/mutation-test,
> and the testing-conventions project (which drives Stryker via its Node API).

## 1. Per-tool patterns

### PIT (Java) — pitest.org

| Aspect | Pattern |
|---|---|
| Fixture model | The *project itself* is the fixture — no synthetic corpus. |
| Invocation | `mvn test-compile org.pitest:pitest-maven:mutationCoverage` mutates all classes matching `targetClasses` and runs `targetTests` globs. |
| Dry run | `--dryRun` gathers coverage + generates mutants without running tests — used in CI pipelines to validate config cheaply. |
| Assertions | Checks the HTML/JSON report's mutation outcomes: Killed / Survived / No coverage / Non viable / Timed Out / Run error. "Non viable" = bytecode invalid (compile-time failure of the mutant); a large non-viable count is treated as a tool bug. |
| Compile-error handling | PIT mutates bytecode and explicitly tries to *minimise non-viable mutations*. Run errors are flagged as "something went wrong" — no hard threshold, but expectation is ~0. |
| Coverage routing | Pre-runs line coverage, maps tests→mutants, and only runs covering tests per mutant. This is the canonical speed optimisation all tools copy. |
| CI integration | Maven/Gradle plugins; arc-mutate adds PR integration. No mocked JDK — runs on the real toolchain. |

### Stryker (JS/TS) — stryker-mutator/stryker-js

| Aspect | Pattern |
|---|---|
| Fixture model | **~25 e2e dirs**, one per real-world scenario: `e2e/test/<scenario>/`. Each is a tiny project with `lib/`, `test/`, `stryker.conf.json`, `package.json`, and a `verify/verify.js` that asserts results. |
| Test pyramid | Unit (1000s) >> integration (test runner plugins, FS, TS compiler) >> e2e (≈25, one-per-use-case). |
| Verify pattern | Every e2e dir has `verify/verify.js` using Chai: calls `expectMetricsJsonToMatchSnapshot()` to compare the generated `mutation.json` against a committed `.snap` file. Also asserts side-effects (e.g. `.stryker-tmp` deleted, log file content, no ERROR/WARN). |
| Coverage routing | `coverageAnalysis: "perTest"` — only tests covering a mutant run against it. `off`/`all`/`perTest` modes. |
| Compile-error handling | Mutates source AST; Stryker injects `// @ts-nocheck` for TS files to suppress type errors introduced by mutations. Does not *assert* a compile-error rate; relies on the mutant being valid JS. |
| Real SDK | Yes — CI installs the real test runner (`vitest`, `jest`, `karma`, `mocha`) per fixture. Fixtures install just the runner; Stryker itself is bundled by the tool, not the fixture. |
| Diff scoping | No native git-diff; implemented by translating changed lines → `--mutate <file>:<start>-<end>` ranges. |

### go-mutesting (Go) — zimmski/go-mutesting

| Aspect | Pattern |
|---|---|
| Fixture model | `example/` dir: `example.go` + `example_test.go` + `sub/` subpackage — a tiny Go package with a test suite. |
| Exec contract | Each mutation tested via an "exec command": replace file → `go test` → report killed. Exit codes: 0=killed, 1=alive, 2=skipped (compile error), >2=unknown. |
| Compile errors | Explicit exit code **2** for compile errors; mutations that don't compile are *skipped* (not counted as killed/survived). No hard rate threshold in the original; the jonbaldie fork adds `--min-msi` quality gates. |
| Coverage routing | None built-in in original (runs all package tests per mutant). jonbaldie fork adds `--per-test` and coverage-aware MSI. |
| CI | Real Go toolchain, no mocking. The fork runs its own mutation-testing CI (`mutation.yml` workflow) — self-hosted dogfooding. |

### mutahunter (LLM-based, language-agnostic)

| Aspect | Pattern |
|---|---|
| Fixture model | `examples/java_maven/` — a real Maven project (BankAccount.java + test). |
| Test command | `mutahunter run --test-command "mvn clean test" --source-path ... --test-path ...`. |
| Compile errors | Reports `🔥 Compile Error Mutants: N` as a distinct category alongside Killed/Survived/Timeout. No <2% threshold; just reported. |
| Coverage routing | "Only mutants whose statements are covered will be executed" — same pre-coverage-filter pattern as PIT/Stryker. |
| Real SDK | Runs the real `mvn` — no mocking. |

### domohuhn/mutation-test (Dart, pub.dev)

| Aspect | Pattern |
|---|---|
| Fixture model | `example/` dir with `config.xml` defining rules, inputs, and test commands. Dogfoods on itself (~30 min run). |
| Mutation engine | Regex-based text replacement, language-agnostic; rules defined in XML. |
| Compile errors | Test commands return exit codes; "incompetent/non-viable mutant" = compile error/crash — counted separately. |
| Coverage routing | "Only mutants whose statements are covered will be executed." Built-in coverage gating. |
| CI | Self-contained binary; runs against the real `dart test` command. |

### testing-conventions (drives Stryker via Node API)

| Aspect | Pattern |
|---|---|
| Fixture model | `tests/fixtures/unit_mutation/<lang>/killed/` and `.../survivors/` — paired fixtures: one with tests that kill every mutant, one with coverage-passing but assertion-light tests that leave survivors. |
| Assertions | Integration test runs the real adapter end-to-end and asserts the survivor count matches expected. E2e also covers failure modes (env unset, broken adapter path). |
| CI install | Fixture installs only `vitest` (the runner); the tool bundles Stryker. CI step: `npm ci` in the fixtures dir before running integration suite. |

## 2. What a fixture looks like for Dart

Based on the patterns above, here is the recommended fixture sizing for `dart-mutant-tests`:

### Small fixture — `fixtures/small/`

| Property | Value |
|---|---|
| Files | 1 lib file (~30 lines), 1 test file (~20 lines) |
| Tests | 3–5 |
| Mutants | ~10–15 |
| Expected mutation score | 100% killed (well-tested) |
| Purpose | Smoke test: CLI runs, report generates, all mutants killed |
| Run time | <5 seconds |

### Medium fixture — `fixtures/medium/`

| Property | Value |
|---|---|
| Files | 3–4 lib files (~100 lines each), 3–4 test files |
| Tests | 15–25 |
| Mutants | ~50–80 |
| Expected outcome | Mixed: some killed, some survived, 1–2 compile errors |
| Purpose | Coverage routing, reporting formats (HTML/JSON/console), partial kill |
| Run time | <30 seconds |

### Large fixture — `fixtures/large/`

| Property | Value |
|---|---|
| Files | 8–10 lib files (realistic Dart package), matching test files |
| Tests | 50+ |
| Mutants | 200+ |
| Expected outcome | Realistic MSI ~60–70%, some survivors, coverage gaps |
| Purpose | Performance, incremental runs, diff-scoped mutation, report scalability |
| Run time | <5 minutes |

## 3. How tools assert "compile error rate < 2%" and "coverage routing works"

### Compile error rate

No tool we surveyed asserts a *hard percentage threshold* on compile errors.
Instead, the established pattern is:

1. **PIT**: Counts "Non viable" mutants. Under normal circumstances, PIT expects
   ~0 non-viable mutants. A large number is flagged as a bug in the tool, not
   the code. No explicit `< N%` assertion in CI, but the report surfaces it.
2. **go-mutesting**: Exit code 2 = compile error; these are *skipped* and
   reported separately. The jonbaldie fork's `--min-msi` gate operates on the
   *covered* MSI denominator, excluding skipped mutants.
3. **mutahunter / mutation_test**: Report "Compile Error Mutants: N" as a
   distinct category. No percentage assertion.
4. **Stryker**: Doesn't explicitly track compile errors (JS mutations rarely
   produce invalid syntax); but verifies via snapshot that the full
   `mutation.json` matches expected results.

**Recommendation for dart-mutant**: Introduce a soft assertion in the E2E
harness: parse the JSON report, compute
`compile_errors / total_mutants`, and assert `< 0.02`. This is stricter than
the surveyed tools but appropriate for a Rust-based mutator that should
produce valid Dart. A higher rate indicates a mutator bug.

### Coverage routing

All surveyed tools implement the same core pattern:

1. **Initial dry run**: Run the full test suite once, collect per-test line
   coverage.
2. **Map tests → mutants**: For each generated mutant, determine which tests
   cover the mutated line.
3. **Per-mutant run**: Only run the covering tests against each mutant (not
   the full suite).
4. **Static mutants**: If a mutation is in module-load code (not in a test
   body), all tests must run — Stryker marks these "static".

**How to assert it in CI**: The medium/large fixture should have a mutant in
a function covered by only 1 of 5 tests. The E2E harness should assert that
the runner executes *only that 1 test* for that mutant (observable via
runner logs or the report's per-mutant test list). This proves the routing
works. A "static" mutant (in `main()` / top-level) should show all tests ran.

## 4. Real SDK vs. mocked toolchain

**Every surveyed tool runs against the real SDK/toolchain in CI.** None mock
the compiler or test runner:

| Tool | Real SDK in CI | Notes |
|---|---|---|
| PIT | Real JDK + Maven/Gradle | |
| Stryker | Real Node + vitest/jest/karma | CI runs `npm ci` in fixtures before suite |
| go-mutesting | Real Go toolchain | Self-hosts: runs mutation CI on itself |
| mutahunter | Real `mvn`/`pytest`/etc | |
| mutation_test | Real `dart test` | Self-contained binary, no deps |

**Recommendation for dart-mutant**: Download the real Dart SDK in CI (via
the `dart-lang/setup-dart` GitHub Action), pin a version, and run the
`dart-mutant-tests` integration suite against it. Mock the toolchain only for
unit tests inside `dart-mutant-core` / `dart-mutant-runner` (e.g. fake
`dart test` exit codes). The E2E layer must use the real SDK — that is the
honest proof the tool works.

## 5. Recommendations for dart-mutant's harness

```
dart-mutant-tests/
├── Cargo.toml
├── fixtures/
│   ├── small/
│   │   ├── lib/calculator.dart      # 1 file, ~30 lines
│   │   └── test/calculator_test.dart
│   ├── medium/
│   │   ├── lib/{math,string,io}.dart
│   │   └── test/{math,string,io}_test.dart
│   └── large/
│       ├── lib/                     # 8-10 files, realistic package
│       └── test/
├── tests/
│   ├── integration.rs    # run dart-mutant over fixtures, assert report JSON
│   ├── coverage_routing.rs  # assert per-mutant test selection
│   └── compile_error_rate.rs # assert < 2% non-viable
└── expected/
    ├── small.json        # expected mutation.json for small fixture
    └── medium.json       # expected (with survivors + compile errors)
```

### Key design decisions

1. **Snapshot-style assertions** (Stryker pattern): Commit expected `mutation.json`
   per fixture. E2E test runs `dart-mutant` and diffs the actual report against
   the expected. Update snapshots deliberately, not automatically.

2. **Paired fixtures** (testing-conventions pattern): `killed/` fixture with
   100% kill rate and `survivors/` fixture with known gaps — proves the tool
   detects both.

3. **Real SDK in CI**: `dart-lang/setup-dart@v1` in the GitHub Actions
   workflow before the integration suite. Pin Dart SDK version for
   reproducibility.

4. **Soft compile-error gate**: Parse report JSON, assert
   `non_viable / total_mutants < 0.02`. Flag in CI output but don't hard-fail
   unless exceeded — match PIT's "expect ~0, investigate if high" philosophy.

5. **Coverage routing test**: Medium fixture with a mutant covered by 1/5
   tests. Assert runner logs show only 1 test executed for that mutant.

6. **Dry-run mode**: Support `dart-mutant --dry-run` (like PIT/Stryker) that
   generates mutants + coverage without running tests — fast CI validation of
   config and mutator correctness.

7. **Self-dogfooding** (go-mutesting pattern): Once stable, run dart-mutant
   on its own Rust codebase in CI — the strongest proof the tool works.

---

*Sources: pitest.org, stryker-mutator.io, stryker-mutator/stryker-js GitHub,
zimmski/go-mutesting GitHub, codeintegrity-ai/mutahunter GitHub,
domohuhn/mutation-test pub.dev, thekevinscott/testing-conventions GitHub.*