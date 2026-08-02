# Goals — dart_mutant

The path to **1.0.0** runs through milestone documents, each following the original GOAL.md pattern: a one-line mission, verifiable acceptance criteria (each with a Test and a Pass), implementation rules, and a **human check** gate at the end.

Each milestone file is independent and self-contained. A milestone moves through three states:

| State | Meaning |
|---|---|
| **draft** | Being written / not yet started |
| **locked** | Criteria agreed; work may begin |
| **signed-off** | Human check passed; milestone complete |

---

## Milestones

| File | Version | Focus | State | Human check |
|---|---|---|---|---|
| [GOAL-0.1.md](GOAL-0.1.md) | 0.1.0 | Original v0.1.0 — shipped, validated 2026-08-02 (143 mutants, 119 killed, MSI 83%, 0 compile errors, 4 binaries, 3 install paths) | ✅ signed-off | 2026-08-02 (validated against real fixture) |
| [GOAL-1.0.md](GOAL-1.0.md) | 1.0.0 | **The harness**: integration test suite, fixtures (small/medium/large + Flutter), CI matrix (Linux + Windows, real SDKs), scale gate, `schemaVersion`, crates.io publishing | 📝 draft | Signed-off live demo (per protocol) |
| [GOAL-2.0.md](GOAL-2.0.md) | 1.1.0 | Flutter per-test routing, incremental polish, ecosystem integrations | 📝 draft | Signed-off live demo (per protocol) |
| [GOAL-3.0.md](GOAL-3.0.md) | 1.2.0 | Hybrid routing, advanced TCE, plugin surface | 📝 draft | Signed-off live demo (per protocol) |

---

## How a milestone is completed

1. Milestone doc is **locked** (criteria agreed — no changes without a human renegotiation).
2. Work proceeds; every acceptance criterion is machine-verifiable.
3. **Human check** (per the protocol decided in wayfinder ticket #3): the agent runs the milestone's acceptance criteria live — harness tests, real project run, reports, install paths. The human spot-checks the receipts and judges real-project fit, then signs off in-session. The agent records the sign-off on the milestone doc.
4. Small failures → rework (criteria do not bend). Big failures → renegotiate criteria, then re-demo.

## Milestone lifecycle

- **Draft** → **Locked**: the human reviews the milestone doc and agrees the criteria are correct and complete. Locking is itself a human act (the agent proposes, the human disposes).
- **Locked** → **Signed-off**: the human check demo passes and the human signs off.
- A milestone is never self-signed-off by the agent.
