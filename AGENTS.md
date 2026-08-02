# dart_mutant — Mutation Testing for Dart

_Last updated: 2026-08-02_

## What this is

dart_mutant is an AST-based mutation testing tool for Dart and Flutter projects. It injects small, deliberate faults (mutants) into Dart source code and runs the test suite against each one. If the tests still pass, the mutant "survived" — revealing a gap in test coverage. The mutation score (MSI) tells you how effective your tests actually are at catching real bugs.

**Why this exists:** Dart has millions of developers (Flutter is a top-3 cross-platform framework), yet the only existing tools are `dart_mutant` by MelbourneDeveloper (15⭐, Dec 2025, v0.1.0, single contributor) and `mutation_test` (regex-based text replacement, not AST-level). Nobody has built a proper AST-based, Dart-native mutation tester with Dart-specific operators for null safety, cascades, async/await, streams, and sealed classes — the constructs that make Dart unique.

**The goal is in [GOAL.md](GOAL.md).** Read it before touching anything. Every acceptance criterion is measurable.

---

## Agent skills

### Issue tracker

Work is tracked as GitHub issues. Skills that read/write the tracker use the `gh` CLI conventions. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context layout: `CONTEXT.md` at the repo root, ADRs under `docs/adr/`. Created lazily by `/domain-modeling` when terms or decisions get resolved; absent files are not a problem. See `docs/agents/domain.md`.

---

## Architecture

```
                    ┌─────────────────────────────────┐
                    │         dart_mutant CLI          │
                    │                                  │
                    │  1. Parse (tree-sitter-dart)      │
                    │  2. Discover mutation points     │
                    │  3. Collect coverage (dart test) │
                    │  4. Build per-test coverage map   │
                    │  5. Inject schemata (compile once)│
                    │  6. Route mutants → covering tests│
                    │  7. Classify (killed/survived/…)  │
                    │  8. TCE equivalent detection      │
                    │  9. Report (JSON/JUnit/HTML)      │
                    └─────────────────────────────────┘
```

**Written in Rust.** Single binary, no service, no database, no language-specific plugin stack. Distributable via `cargo install`, Homebrew, or a pre-built binary — same deployment story as gomutants and mewt.

### Why Rust

- tree-sitter has first-class Rust bindings (`tree-sitter-dart` grammar exists)
- Single static binary — no runtime dependency (unlike Python/Node tools)
- Parallel mutation execution is trivial (Rayon)
- The existing `dart_mutant` (MelbourneDeveloper) is also Rust — validates the approach

### Why tree-sitter (not the Dart analyzer)

- The Dart analyzer is a Dart package — would require shelling out to `dart` for parsing
- tree-sitter-dart provides a native Rust AST with incremental parsing support
- tree-sitter is what mewt, togi, and mutahunter all use — proven approach
- The Dart analyzer can be used as a *verification* step (compile-check mutants) without being the primary parser

---

## Monorepo layout

This is a Cargo workspace, not a pnpm/Turbo monorepo.

| Crate | Type | Purpose |
|---|---|---|
| `dart-mutant` | **binary** | The CLI tool — entry point, argument parsing, orchestration |
| `dart-mutant-core` | library | AST parsing, mutation operators, schemata generation |
| `dart-mutant-runner` | library | Test execution, coverage routing, timeout management, parallel scheduler |
| `dart-mutant-report` | library | Stryker JSON, JUnit XML, HTML report, console summary |
| `dart-mutant-tce` | library | Trivial Compiler Equivalence — Dart kernel/bytecode comparison |
| `dart-mutant-tests` | integration tests | End-to-end test suite against real Dart projects |

```
dart-mutant/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── dart-mutant/              # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli.rs            # clap argument parsing
│   │       └── config.rs        # YAML / CLI config merging
│   ├── dart-mutant-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs         # tree-sitter-dart wrapper
│   │       ├── ast.rs            # AST node types
│   │       ├── mutant.rs         # Mutant struct, MutantId, mutation point
│   │       ├── operators/
│   │       │   ├── mod.rs        # Mutator trait
│   │       │   ├── arithmetic.rs # AOR, AOD, AOI
│   │       │   ├── relational.rs # ROR
│   │       │   ├── logical.rs    # LOR, LCR
│   │       │   ├── conditional.rs # COR, negate
│   │       │   ├── statement.rs  # SDL
│   │       │   ├── return.rs     # RVR
│   │       │   ├── loop.rs       # inc/dec
│   │       │   ├── null_safety.rs # ??, !, ?.
│   │       │   ├── cascade.rs    # .. → .
│   │       │   ├── async_await.rs # remove await
│   │       │   ├── stream.rs     # stream mutations
│   │       │   └── sealed_class.rs # switch exhaustiveness
│   │       └── schemata.rs      # Mutant schemata generation
│   ├── dart-mutant-runner/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── coverage.rs       # dart test --coverage → per-test map
│   │       ├── scheduler.rs      # Parallel mutant execution (Rayon)
│   │       ├── timeout.rs        # Adaptive per-mutant timeout
│   │       └── cache.rs          # Content-addressed incremental cache
│   ├── dart-mutant-report/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── stryker_json.rs   # Stryker mutation-testing-elements schema
│   │       ├── junit_xml.rs      # JUnit XML
│   │       ├── html.rs            # Self-contained HTML report
│   │       └── console.rs        # Terminal summary
│   └── dart-mutant-tce/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── kernel_compare.rs # dart compile kernel → compare bytecode
├── tests/
│   ├── fixtures/                  # Sample Dart projects for E2E tests
│   │   ├── small/                # <10 files
│   │   ├── medium/               # 10-50 files
│   │   └── large/                # 50+ files
│   └── integration/
│       ├── compilation_rate.rs   # <2% compile failure
│       ├── operator_count.rs     # ≥6 Dart-specific, ≥7 generic
│       ├── schemata.rs           # compile once, not N
│       ├── coverage_routing.rs   # avg tests < total
│       ├── timeout.rs            # infinite loop → TIMEOUT
│       ├── tce.rs                # ≥80% equivalent detection
│       └── report_formats.rs     # 5/5 valid formats
├── docs/
│   └── adr/                      # Architecture Decision Records
└── .github/
    └── workflows/
        └── ci.yml                # cargo test + cargo clippy + cargo fmt --check
```

---

## Development commands

```bash
# Build
cargo build                          # debug build
cargo build --release                # release binary

# Test
cargo test                           # all unit + integration tests
cargo test --package dart-mutant-core # core operators only
cargo test --test integration        # E2E suite against fixtures

# Lint
cargo clippy -- -D warnings          # zero warnings
cargo fmt -- --check                 # formatting check

# Run locally against a Dart project
cargo run --release -- --path /path/to/dart/project

# Run with specific options
cargo run --release -- --path ./my-app --threshold 80 --parallel 8 --format html,json,junit

# Incremental mode (only changed files)
cargo run --release -- --path ./my-app --incremental --base-ref main

# TCE equivalent detection
cargo run --release -- --path ./my-app --detect-equivalent
```

### Prerequisites for integration tests

- Dart SDK 3.x installed and on `PATH` (`dart --version`)
- The test fixtures in `tests/fixtures/` are real Dart packages with `pubspec.yaml` and test suites

---

## Environment

dart_mutant reads its configuration from (in priority order):

1. **CLI flags** (highest priority)
2. **`.dart_mutant.yml`** in the project root
3. **Defaults** (lowest priority)

```yaml
# .dart_mutant.yml example
test_command: "dart test"          # or "flutter test"
source_path: "lib/"                # default: lib/
exclude:
  - "*.g.dart"                    # generated files
  - "*.freezed.dart"
  - "*.mocks.dart"
threshold: 80                      # MSI threshold for CI gate (0-100)
parallel: 8                        # parallel workers (default: CPU count)
timeout_coefficient: 3.0           # adaptive timeout = baseline × coefficient
detect_equivalent: false           # enable TCE (opt-in)
incremental: false                 # only mutate changed files
base_ref: "main"                   # git ref for incremental mode
format:                            # output formats
  - console                        # always on
  - html                           # self-contained HTML report
  - json                            # Stryker-compatible JSON
  - junit                           # JUnit XML
operators:                         # restrict operators (default: all)
  - AOR
  - ROR
  - NullSafety
  - Cascade
  - AsyncAwait
```

---

## Installation

### For users

#### Homebrew (macOS / Linux)

```bash
brew install SulthanZahran1/tap/dart_mutant
```

#### Pre-built binary (Linux / macOS / Windows)

```bash
# Linux (x86_64)
curl -L https://github.com/SulthanZahran1/dart-mutant/releases/latest/download/dart_mutant-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv dart_mutant /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/SulthanZahran1/dart-mutant/releases/latest/download/dart_mutant-aarch64-apple-darwin.tar.gz | tar xz
sudo mv dart_mutant /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/SulthanZahran1/dart-mutant/releases/latest/download/dart_mutant-x86_64-apple-darwin.tar.gz | tar xz
sudo mv dart_mutant /usr/local/bin/

# Windows (PowerShell)
curl -L https://github.com/SulthanZahran1/dart-mutant/releases/latest/download/dart_mutant-x86_64-pc-windows-msvc.zip -o dart_mutant.zip
Expand-Archive dart_mutant.zip -DestinationPath C:\Program Files\dart_mutant
# Add C:\Program Files\dart_mutant to your PATH
```

#### Cargo (from source)

```bash
cargo install dart-mutant
```

#### Verify installation

```bash
dart_mutant --version
dart_mutant --help
```

#### Prerequisites

- **Dart SDK 3.0+** on `PATH` (`dart --version`). dart_mutant shells out to `dart test` (or `flutter test`) to run your test suite.
- **Flutter SDK** if your project uses `flutter test` instead of `dart test`.
- **git** if using `--incremental` mode (reads `git diff` to scope mutations to changed lines).
- No Rust toolchain required — the binary is statically linked (musl on Linux).

### For AI agents

AI agents (Claude Code, Codex, Cursor, Hermes, etc.) typically install dev tools as a one-shot shell command during session setup. The fastest path:

#### One-shot install (auto-detect platform)

```bash
# Detects OS + arch, downloads the right binary, installs to /usr/local/bin
curl -fsSL https://raw.githubusercontent.com/SulthanZahran1/dart-mutant/main/scripts/install.sh | bash
```

This script:
1. Detects OS (`linux` / `macos` / `windows`) and architecture (`x86_64` / `aarch64`)
2. Downloads the matching pre-built binary from GitHub Releases
3. Installs to `/usr/local/bin/dart_mutant` (or `$HOME/.local/bin` if no sudo)
4. Verifies with `dart_mutant --version`
5. Exits 0 on success, non-zero on failure

#### Install via Cargo (if Rust toolchain is available)

```bash
cargo install dart-mutant 2>&1 | tail -1
```

#### Programmatic usage from an agent

Agents should use `--format json` for machine-readable output and parse the JSON for mutation scores, survivor counts, and per-file breakdowns:

```bash
# Run mutation testing, get JSON output
dart_mutant --path ./my-dart-project --format json --quiet --threshold 80

# JSON output schema (stdout):
{
  "mutationScore": 72.5,
  "mutationCoverage": 85.0,
  "total": 500,
  "killed": 310,
  "survived": 118,
  "timeout": 12,
  "equivalent": 30,
  "notCovered": 25,
  "compileError": 5,
  "threshold": 80,
  "passed": false,
  "files": [
    {
      "path": "lib/src/math_utils.dart",
      "mutationScore": 90.0,
      "killed": 18,
      "survived": 2,
      "total": 20
    }
  ]
}
```

```bash
# Agent-friendly: JSON only, no color, no progress bars
dart_mutant --path ./my-dart-project --format json --quiet --no-color

# Check exit code for CI gate
echo $?  # 0 = MSI ≥ threshold, 1 = below threshold, 2 = error
```

#### Agent workflow: "find weak tests and fix them"

```bash
# Step 1: Run mutation testing on changed files only
dart_mutant --incremental --base-ref origin/main --format json --quiet

# Step 2: Parse JSON, find files with surviving mutants
# Step 3: For each survivor, read the mutation diff and the covering test
# Step 4: Write a new test case that would kill the survivor
# Step 5: Re-run only that mutant to verify it's now killed
dart_mutant --mutant <mutant-id> --format json --quiet
```

---

## Usage

### Quick start

```bash
cd my-dart-project
dart_mutant
```

That's it. dart_mutant auto-detects:
- `lib/` as the source directory
- `dart test` as the test command (or `flutter test` if `flutter` is on `PATH` and a `pubspec.yaml` with Flutter SDK is present)
- All mutation operators (generic + Dart-specific)
- CPU count for parallel workers
- `.dart_mutant.yml` if present in the project root

### First run

```bash
$ cd my-flutter-app
$ dart_mutant

dart_mutant v0.1.0 — Mutation Testing for Dart

Parsing lib/............................ 142 files
Discovering mutation points............ 1,247 mutants found
Collecting coverage.................... dart test --coverage
Building per-test coverage map......... 312 tests mapped
Injecting schemata (compile once)..... done in 3.2s
Running mutants (8 parallel)......... ████████████████░░░░ 847/1247

Results:
  Killed:       689  (55.3%)
  Survived:     158  (12.7%)
  Timeout:      12   (1.0%)
  Equivalent:   43   (excluded)
  Not covered:  298  (excluded)
  Compile error: 7   (0.6%)

  Mutation Score (MSI): 80.2%
  Mutation Coverage:   71.5%
  Threshold: 80%       ✅ PASSED

Reports:
  → mutation-reports/mutation-report.html
  → mutation-reports/mutation-report.json
  → mutation-reports/mutation-results.xml
```

### Common commands

```bash
# Full run with all reports
dart_mutant --format html,json,junit

# Set a CI threshold (exit 1 if below)
dart_mutant --threshold 80

# Only test mutants on lines changed vs main branch
dart_mutant --incremental --base-ref main

# Run a specific mutant by ID (for iterative fixing)
dart_mutant --mutant 42

# Restrict to specific operators
dart_mutant --operators AOR,ROR,NullSafety,Cascade

# Limit mutations for quick feedback
dart_mutant --sample 50

# Enable equivalent-mutant detection (slower, more accurate score)
dart_mutant --detect-equivalent

# Use Flutter test instead of dart test
dart_mutant --test-command "flutter test"

# Exclude specific files/globs
dart_mutant --exclude "lib/generated/**,lib/l10n/**"

# Adjust parallelism
dart_mutant --parallel 4

# Quiet mode (CI-friendly, minimal output)
dart_mutant --quiet --format json

# Custom timeout coefficient (default 3.0)
dart_mutant --timeout-coefficient 5.0
```

### Configuration file

Create `.dart_mutant.yml` in your project root:

```yaml
# .dart_mutant.yml
test_command: "flutter test"
source_path: "lib/"
exclude:
  - "*.g.dart"
  - "*.freezed.dart"
  - "*.mocks.dart"
  - "lib/generated/**"
  - "lib/l10n/**"
threshold: 80
parallel: 8
timeout_coefficient: 3.0
detect_equivalent: false
incremental: false
base_ref: "main"
format:
  - console
  - html
  - json
  - junit
operators:
  - AOR
  - ROR
  - LOR
  - LCR
  - COR
  - SDL
  - RVR
  - INC
  - NullSafety
  - NullAssert
  - OptionalChaining
  - Cascade
  - AsyncAwait
  - SealedExhaustiveness
  - StreamMutation
```

CLI flags always override `.dart_mutant.yml`, which always overrides defaults.

### Reading the report

#### HTML report

Open `mutation-reports/mutation-report.html` in any browser. It's self-contained — no external CSS, JS, or internet required. Shows:
- Per-file mutation score heatmap (green = high kill rate, red = low)
- Per-mutant detail: original code, mutated code, diff, status, covering tests
- Filterable by status (killed / survived / timeout / equivalent / not covered)

#### JSON report (Stryker-compatible)

Validates against the [mutation-testing-elements](https://github.com/stryker-mutator/mutation-testing-elements) JSON schema. Compatible with the Stryker dashboard at `https://dashboard.stryker-mutator.io`.

#### JUnit XML

Standard JUnit format for CI test result panels. Each mutant appears as a test case with pass/fail status. Shows up natively in GitHub Actions, GitLab CI, Azure Pipelines, and Jenkins.

### Interpreting results

| Status | What it means | What to do |
|---|---|---|
| **KILLED** | Your tests caught the mutation. | Nothing — this is good. |
| **SURVIVED** | Your tests didn't catch the mutation. | Read the diff, understand what changed, write a test that would fail if that mutation were applied. |
| **TIMEOUT** | The mutation caused an infinite loop or hang. | The mutation likely broke a loop condition. Add a test with boundary inputs. |
| **EQUIVALENT** | The mutation produces identical bytecode. | Nothing — this mutant is unkillable. It's excluded from your score. |
| **NOT_COVERED** | No test exercises the mutated line. | Add a test that reaches this code path. |
| **COMPILE_ERROR** | The mutation produced invalid Dart. | If this is >2% of total, file a bug — the tree-sitter grammar may need updating. |

### Iterative workflow (the recommended loop)

```bash
# 1. Run mutation testing on changed files
dart_mutant --incremental --base-ref main --format html

# 2. Open the HTML report, filter by "survived"
open mutation-reports/mutation-report.html

# 3. For each survivor:
#    - Read the mutation diff (what changed)
#    - Read the covering test (why it didn't catch it)
#    - Write a new test case that asserts the mutated behavior

# 4. Re-run only that mutant to verify you killed it
dart_mutant --mutant <mutant-id>

# 5. Re-run the full incremental to confirm your score improved
dart_mutant --incremental --base-ref main
```

---

## Core principles — the CRITICAL RULES

### 1. CRITICAL: AST-only mutations — never regex

Every mutation is produced by walking a tree-sitter-dart AST and transforming tree nodes. **Never** mutate source text via regex or string replacement. Regex mutation produces invalid syntax, false positives, and cannot target Dart-specific constructs.

```rust
// ❌ NEVER — regex mutation
let mutated = source.replace("+", "-");

// ✅ AST-based mutation
let tree = parser.parse(source, None);
let mut visitor = MutationVisitor::new();
visitor.walk(&tree);
let mutant = visitor.apply_mutation(mutation_point);
```

**WHY:** The existing `mutation_test` pub.dev package uses regex — and it produces compile errors, misses language constructs, and can't distinguish `+` in a string literal from `+` in an expression. AST parsing is the baseline requirement for any serious mutation tester (PIT, Stryker, Infection, gomutants, mutant-ruby — all AST-based).

### 2. Compile once — mutant schemata

The tool does **not** recompile per-mutant. All mutations are injected into a single compilation as conditional branches, switched at runtime via an environment variable (`DART_MUTANT_ID`). This is the **mutant schemata** technique (same as muter for Swift, kanly for Go).

```
Source:      fn add(a, b) => a + b
Schemata:    fn add(a, b) => match env("DART_MUTANT_ID") {
               "0" => a - b,   // AOR mutant 0
               "1" => a * b,   // AOR mutant 1
               _    => a + b,   // original
             }
```

Compile once. Run N times with different `DART_MUTANT_ID`. N test runs, 1 compilation.

**WHY:** Compilation is the dominant cost in mutation testing. A 500-mutant run with per-mutant compilation = 500 compilations × ~3s each = 25 minutes just compiling. Schemata reduces that to 1 compilation (~3s) + 500 test runs. This is the difference between "practical" and "unusable."

### 3. Route mutants to covering tests only

Each mutant runs only against the tests whose coverage touches the mutated line — not the entire test suite. Coverage is collected once via `dart test --coverage`, then a per-test → per-line map routes each mutant to its covering tests.

**WHY:** If a mutant is on line 42 of `math_utils.dart` and only `math_utils_test.dart` lines 15-20 cover that line, running the entire suite (including `auth_test.dart`, `api_test.dart`, …) for every mutant is pure waste. Per-test coverage routing cuts wall-clock by 5-10× on typical projects (gomutants measured this — they call it "per-test coverage routing" and it's their primary performance win).

### 4. Every mutant is classified — no ambiguity

Every mutant ends up in exactly one state: `KILLED`, `SURVIVED`, `TIMEOUT`, `EQUIVALENT`, `NOT_COVERED`, or `COMPILE_ERROR`. No mutant is left unclassified or silently dropped. The mutation score denominator is `KILLED + SURVIVED + TIMEOUT` (not `EQUIVALENT`, not `NOT_COVERED`, not `COMPILE_ERROR`).

```rust
pub enum MutantStatus {
    Killed,        // test suite failed
    Survived,      // test suite passed
    Timeout,       // exceeded adaptive timeout
    Equivalent,    // TCE detected identical bytecode
    NotCovered,    // no test covers the mutated line
    CompileError,  // mutation produces invalid code (should be <2%)
}

pub fn mutation_score(results: &[MutantResult]) -> f64 {
    let relevant: Vec<_> = results.iter()
        .filter(|r| matches!(r.status, Killed | Survived | Timeout))
        .collect();
    let killed = results.iter()
        .filter(|r| matches!(r.status, Killed))
        .count();
    killed as f64 / relevant.len() as f64 * 100.0
}
```

**WHY:** A mutation score that includes equivalent mutants is artificially deflated (they can never be killed). A score that includes not-covered mutants is misleading (they weren't tested at all). The denominator must be "mutants that were actually tested" — killed + survived + timeout.

### 5. Reports must integrate with existing ecosystems

The Stryker JSON format is the de facto standard for mutation testing reports (used by StrykerJS, Stryker.NET, Stryker4s, gomutants, muter). JUnit XML is the standard for CI test result panels. The HTML report must be self-contained (no external CSS/JS) and openable directly in a browser.

```bash
# Stryker-compatible JSON — validates against mutation-testing-elements schema
dart_mutant --format json -o mutation-report.json

# JUnit XML — for GitHub/GitLab/Azure CI panels
dart_mutant --format junit -o mutation-results.xml

# HTML — self-contained, no external deps
dart_mutant --format html -o mutation-report.html

# Exit codes for CI gate
dart_mutant --threshold 80    # exit 0 if MSI ≥ 80%, exit 1 if below
```

**WHY:** A mutation testing tool that produces only a custom format is useless in CI. Stryker-compatible JSON means instant integration with the mutation-testing-elements dashboard. JUnit XML means instant integration with GitHub Actions test results. This is table stakes.

### 6. Performance is a feature

Mutation testing is inherently expensive (N mutants × test suite runtime). If the tool is slow, nobody uses it. The performance budget is non-negotiable:

| Metric | Target |
|---|---|
| Cold run (500 mutants, 50 files) | < 10 minutes |
| Warm rerun (same source + tests) | < 30 seconds |
| Compilation calls per run | 1 (mutant schemata) |
| Tests run per mutant | < total test count (coverage routing) |
| Parallel scaling | Linear with `--jobs` |
| Incremental mode | Only mutants on changed lines |

**WHY:** gomutants measured that warm reruns with caching are 120-150× faster than cold runs (46 min → 19s on `prometheus/tsdb`). If we can't achieve similar speedups, the tool won't be used in CI — and a mutation tester not used in CI is dead.

---

## Mutation operators

### Generic operators (≥7 required)

| Operator | Code | Mutations |
|---|---|---|
| Arithmetic Operator Replacement | AOR | `+` → `-`, `*` → `/`, `%` → `*` |
| Arithmetic Operator Deletion | AOD | `a + b` → `a` |
| Arithmetic Operator Insertion | AOI | `a` → `a + 1` |
| Relational Operator Replacement | ROR | `>` → `>=`, `==` → `!=`, `<` → `<=` |
| Logical Operator Replacement | LOR | `&&` → `\|\|` |
| Logical Constant Replacement | LCR | `true` → `false` |
| Conditional Operator Replacement | COR | `if (x)` → `if (!x)` |
| Statement Deletion | SDL | Remove statement |
| Return Value Replacement | RVR | Replace return with zero/empty |
| Increment/Decrement | INC | `i++` → `i--` |

### Dart-specific operators (≥6 required — the differentiator)

| Operator | Mutation | Why it matters |
|---|---|---|
| **NullSafety** | `a ?? b` → `a` (remove fallback) | Dart's `??` is the core null-safety operator. Removing it tests whether the suite catches missing null handling. |
| **NullAssert** | `a!` → `a` (remove force-unwrapping) | `!` is Dart's null assertion. Removing it tests whether the suite catches unsafe null access. |
| **OptionalChaining** | `a?.b` → `a.b` (remove safe access) | `?.` is Dart's safe member access. Removing it tests whether the suite catches null propagation gaps. |
| **Cascade** | `a..b()` → `a.b()` (cascade → dot) | `..` is Dart-unique (method cascade). Converting to `.` changes the return value — tests whether the suite catches cascade misuse. |
| **AsyncAwait** | `await f()` → `f()` (remove await) | Removing `await` changes a Future's resolution. Tests whether the suite catches missing async synchronization. |
| **SealedExhaustiveness** | Remove a `case` from `switch` on a `sealed class` | Dart 3 sealed classes + exhaustive switch. Removing a branch tests whether the suite catches incomplete pattern matching. |
| **StreamMutation** | `stream.first` → `stream.last` | Streams are core to Dart/Flutter state management. Swapping stream properties tests whether the suite catches wrong stream consumption. |

---

## Mutant classification

| Status | Meaning | In mutation score? |
|---|---|---|
| **KILLED** | Test suite failed → mutation detected | ✅ numerator + denominator |
| **SURVIVED** | Test suite passed → mutation undetected | ✅ denominator only |
| **TIMEOUT** | Mutation caused infinite loop/hang | ✅ numerator + denominator |
| **EQUIVALENT** | TCE detected identical bytecode | ❌ excluded |
| **NOT_COVERED** | No test covers the mutated line | ❌ excluded |
| **COMPILE_ERROR** | Mutation produces invalid code | ❌ excluded (should be <2%) |

**Mutation Score (MSI)** = `KILLED / (KILLED + SURVIVED + TIMEOUT) × 100`

**Mutation Coverage** = `(KILLED + SURVIVED) / (KILLED + SURVIVED + NOT_COVERED) × 100`

---

## CI/CD integration

```yaml
# .github/workflows/mutation.yml
name: Mutation Testing
on:
  pull_request:
    branches: [main]
jobs:
  mutation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dart-lang/setup-dart@v1
        with:
          sdk: stable
      - name: Install dart_mutant
        run: |
          curl -L https://github.com/SulthanZahran1/dart-mutant/releases/latest/download/dart_mutant-x86_64-unknown-linux-gnu.tar.gz | tar xz
          sudo mv dart_mutant /usr/local/bin/
      - name: Run mutation tests (incremental)
        run: dart_mutant --incremental --base-ref origin/main --threshold 80 --format json,junit,html --quiet
      - name: Upload reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: mutation-report
          path: mutation-reports/
```

---

## Common issues

| Issue | Check |
|---|---|
| "No mutants found" | Ensure `source_path` points to `lib/` (default). Generated files (`*.g.dart`, `*.freezed.dart`) are excluded by default. |
| Compilation failure rate > 2% | The tree-sitter-dart grammar may not handle a Dart 3 feature. Check if the feature is in the grammar's `grammar.js`. |
| All mutants TIMEOUT | `timeout_coefficient` too low. Increase in config or via `--timeout-coefficient 5.0`. |
| Mutation score is 0% | Test suite may not be covering the mutated code. Check `NOT_COVERED` count. Run `dart test --coverage` separately to verify. |
| TCE marks everything as equivalent | The Dart kernel comparison may be too coarse. Check `dart compile kernel` output for both original and mutant. |
| `flutter test` not working | Use `--test-command "flutter test"` explicitly. dart_mutant defaults to `dart test`. |
| Slow cold run | Ensure mutant schemata is enabled (default). Check `--parallel` matches CPU count. Use `--sample N` for quick feedback. |

---

## Security

- dart_mutant executes `dart test` (or `flutter test`) as a subprocess. It does not execute arbitrary user code beyond what the test suite already runs.
- Mutant schemata injects code into the test compilation only — production builds are never touched. The `--incremental` and `--base-ref` flags use `git diff` (read-only).
- No network calls. No telemetry. No data leaves the machine.

---

## The standard

A mutation testing tool is only useful if developers actually run it. That means: it must be fast enough for CI, accurate enough to trust, and integrate with the tools developers already use (Stryker dashboard, JUnit CI panels, HTML reports). If the tool produces false positives (incompetent mutants), developers will ignore all survivors. If it's slow, it won't run in CI. If it produces a custom report format nobody recognizes, it won't be adopted. Every decision in this repo should be weighed against: **would a Dart/Flutter team actually use this in their daily workflow?**
