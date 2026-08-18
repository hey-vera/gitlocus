<!-- SPDX-License-Identifier: Apache-2.0 -->
# AGENTS.md

Instructions for coding agents working in this repository. Follows the
[AGENTS.md](https://agents.md) convention.

If you are an agent reading this: you are welcome here. This project exists
because agent-produced work is valuable and needs a better way to be evaluated,
not because it needs to be kept out. What follows is what the gate will check.

## What this project is

GitLocus is a contribution-provenance and policy layer. Five types carry the
model — Actor, Contribution, Evidence, Policy, Verdict — and the normative
definitions live in [`spec/README.md`](spec/README.md). Read that before changing
anything in `crates/locus-core`.

## Build and test

```bash
cargo build --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

All four must pass. They are the same commands CI runs; there is no CI-only step.

To see what the gate will say about a change before pushing:

```bash
cargo run -p locus-cli -- policy check --policy .gitlocus/policy.yml
```

## Rules that are not negotiable

These are invariants of the model, not style preferences. A change that breaks one
is wrong even if it compiles and the tests pass — in which case the tests are also
wrong.

1. **Assessed evidence must never satisfy a requirement.** Not with a high score,
   not with a confidence threshold, not behind a flag. If you find yourself adding
   a way for a model's judgement to unblock a merge, stop: that is the exact
   failure this project exists to prevent.
2. **Evidence bound to a different revision must never count.** This is what stops
   a green result from before a force-push being credited to the code that
   replaced it.
3. **`inconclusive` is unmet, never a pass.**
4. **Verdicts are pure.** No clock, no network, no ambient state, and no
   dependence on the order of the evidence array. There is a test for this.
5. **No AI-authorship detection.** See
   [ADR 0002](docs/adr/0002-no-ai-authorship-detection.md). Do not add heuristics
   that guess whether code was model-written.

## Conventions

- Every source file starts with `// SPDX-License-Identifier: Apache-2.0`.
- Public items carry doc comments; `missing_docs` is a warning and CI denies warnings.
- Comments explain **why**, not what. If a comment restates the code, delete it.
- Tests are named as the claim they make — `assessed_evidence_never_satisfies_a_requirement`,
  not `test_policy_3`.
- Changes to `spec/` require matching changes to the conformance suite in
  `crates/locus-core/tests/conformance.rs`. The policy enforces this.

## Where things are

| path | what |
|---|---|
| `spec/` | Normative model and JSON Schemas |
| `crates/locus-core/` | Reference implementation of the model |
| `crates/locus-cli/` | The `locus` binary |
| `.gitlocus/policy.yml` | The policy this repository runs on itself |
| `docs/adr/` | Why decisions were made |

## Submitting work

Read [CONTRIBUTING.md](CONTRIBUTING.md) first — in particular the AI-contribution
policy, which applies to you.

Two things matter more than anything else in your pull request:

- **A named human must be answerable for it.** Sign off with
  `git commit -s` (DCO). An agent contribution with no responsible operator will
  not be reviewed.
- **Say what you actually verified.** "Tests pass" when you did not run them is
  worse than saying nothing, because it spends a maintainer's attention on
  checking a claim you already knew was unfounded. Under-claiming is free;
  over-claiming costs you the benefit of the doubt on everything else.

Do not open a pull request that changes `.github/workflows/**` or `.gitlocus/**`
unless that is the explicit purpose of the change. Those paths require maintainer
standing and will be blocked.
