# dart_mutant — Mutation Testing for Dart & Flutter

**The AST-based mutation testing tool for Dart.** `dart_mutant` injects deliberate faults (mutants) into your Dart and Flutter source code, runs your tests against each one, and reports which mutants your tests failed to catch — giving you a **Mutation Score Indicator (MSI)** that is far more meaningful than line coverage. If you're looking for **dart mutation testing** or **flutter mutation testing**, this is the tool that finds the tests that miss real bugs.

Dart had almost no mutation-testing supply (the only existing tools were a 15-star proof-of-concept and a regex-based replacer). `dart_mutant` is the first proper AST-level, Dart-native mutation tester — with operators for the constructs that make Dart unique: **null safety, cascades, async/await, streams, and sealed classes**.

## Install

### One-shot (any OS, auto-detects platform)

```bash
curl -fsSL https://raw.githubusercontent.com/SulthanZahran1/dart-mutant/main/scripts/install.sh | bash
```

### Homebrew (macOS / Linux)

```bash
brew install SulthanZahran1/tap/dart_mutant
```

### Cargo (from crates.io)

**`dart_mutant` 1.0.0 is published on [crates.io](https://crates.io/crates/dart-mutant).**

```bash
cargo install dart-mutant
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/SulthanZahran1/dart-mutant/releases):

| Platform | Asset |
|---|---|
| Linux (x86_64, static musl — runs anywhere) | `dart_mutant-x86_64-unknown-linux-musl.tar.gz` |
| macOS (Apple Silicon) | `dart_mutant-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `dart_mutant-x86_64-apple-darwin.tar.gz` |
| Windows (x86_64) | `dart_mutant-x86_64-pc-windows-msvc.zip` |

**Prerequisite:** Dart SDK 3.0+ on `PATH` (`dart --version`). Flutter SDK if you use `flutter test`.

## Quick start

```bash
cd my-dart-project
dart_mutant
```

That's it. It auto-detects `lib/` as the source root, `dart test` (or `flutter test`), all 17 mutation operators, and CPU count for parallel workers. Results land in `mutation-reports/` (HTML + JSON + JUnit XML) and the console.

```bash
# CI gate — exit 0 if MSI ≥ 80, exit 1 otherwise
dart_mutant --threshold 80

# Machine-readable output for CI dashboards / AI agents
dart_mutant --format json --quiet

# Only mutants on lines changed vs main (incremental)
dart_mutant --incremental --base-ref main

# Equivalent-mutant detection via Dart kernel bytecode comparison
dart_mutant --detect-equivalent

# Iterative workflow: re-run one mutant after writing a killing test
dart_mutant --mutant 42
```

## Why it's different

| Feature | dart_mutant | Typical mutation tools |
|---|---|---|
| **Dart-specific operators** | 7 — null safety (`??`, `!`, `?.`), cascade (`..`→`.`), async/await, streams, sealed-class exhaustiveness | generic operators only |
| **Generic operators** | 10 — AOR, AOD, AOI, ROR, LOR, LCR, COR, SDL, RVR, loop inc/dec | varies |
| **Coverage routing** | runs only the tests that cover the mutated line (per-test-file routing from Dart VM coverage JSON) | runs full suite per mutant |
| **Equivalent detection** | TCE — compiles original + mutant to Dart kernel and compares bytecode | rarely offered |
| **Timeouts** | adaptive — baseline duration × coefficient, never a fixed global timeout | fixed or missing |
| **Reports** | Stryker-compatible JSON (schema v2), JUnit XML, self-contained HTML heatmap | often just text |
| **Speed** | parallel (Rayon), content-addressed SHA-256 cache for warm reruns | sequential |
| **Agent-friendly** | `--format json --quiet` = pure JSON on stdout, exit codes 0/1/2 | — |

## Mutation operators

**Generic (10):** AOR, AOD, AOI, ROR, LOR, LCR, COR, SDL, RVR, loop inc/dec

**Dart-specific (7):** NullSafety (`??` removal), NullAssert (`!` removal), OptionalChaining (`?.`→`.`), Cascade (`..`→`.`), AsyncAwait (remove `await`), StreamMutation, SealedClassExhaustiveness

Restrict with `--operators AOR,ROR,NullSafety` or via `.dart_mutant.yml`.

## Configuration

CLI flags > `.dart_mutant.yml` > defaults.

```yaml
# .dart_mutant.yml
test_command: "flutter test"
source_path: "lib/"
exclude:
  - "*.g.dart"
  - "*.freezed.dart"
  - "*.mocks.dart"
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
  - NullSafety
  - Cascade
  - AsyncAwait
```

## Interpreting results

| Status | Meaning | What to do |
|---|---|---|
| **KILLED** | tests caught the mutation | nothing — good |
| **SURVIVED** | tests missed it | read the diff, write a test that fails under the mutation |
| **TIMEOUT** | mutation caused a hang | add boundary-input tests for loop conditions |
| **EQUIVALENT** | bytecode-identical — unkillable | excluded from score; no action |
| **NOT_COVERED** | no test touches the line | add a test that reaches the code path |
| **COMPILE_ERROR** | invalid Dart produced | should be <2% — if higher, file an issue |

## Development

```bash
cargo build --release
cargo test                # 155 unit tests
cargo clippy -- -D warnings
```

Architecture and conventions: [AGENTS.md](AGENTS.md). Verifiable goals: [GOAL.md](GOAL.md).

## Roadmap

- [ ] Flutter widget-test coverage routing (per-test-file JSON is already parsed; widget test trees need special handling)
- [ ] `--since` git-diff scoping for PR-based workflows
- [ ] Stryker dashboard upload (`--dashboard`)
- [ ] Homebrew tap auto-update via release workflow

## License

MIT
