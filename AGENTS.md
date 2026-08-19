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
anything in `crates/gitlocus-core`.

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
cargo run -p gitlocus-cli -- policy check --policy .gitlocus/policy.yml
```

## Rules that are not negotiable

These are invariants of the model, not style preferences. A change that breaks one
is wrong even if it compiles and the tests pass — in which case the tests are also
wrong.

1. **Assessed evidence must never satisfy a requirement.** Not with a high score,
   not with a confidence threshold, not behind a flag. If you find yourself adding
   a way for a model's judgement to unblock a merge, stop: that is the exact
   failure this project exists to prevent. If you are about to argue that models
   are reliable enough now, read
   [ADR 0005](docs/adr/0005-evidence-classes-survive-better-models.md) first — it
   was written to answer exactly that argument, and the answer does not depend on
   model quality.
2. **Evidence bound to a different revision must never count.** This is what stops
   a green result from before a force-push being credited to the code that
   replaced it.
3. **`inconclusive` is unmet, never a pass.**
4. **Verdicts are pure.** No clock, no network, no ambient state, and no
   dependence on the order of the evidence array. There is a test for this. Purity
   is also what makes verdicts content-addressable and therefore cacheable at
   swarm scale — do not trade it away for convenience.
5. **No AI-authorship detection.** See
   [ADR 0002](docs/adr/0002-no-ai-authorship-detection.md). Do not add heuristics
   that guess whether code was model-written. Recording what a named human
   *declares* is a different thing and is allowed —
   [ADR 0008](docs/adr/0008-authorship-is-declared-not-detected.md).
6. **A signer is never read from input.** `Evidence::signer` is
   `skip_deserializing` on purpose. If you make it deserializable, forging trusted
   CI identity becomes a matter of typing it into a JSON file.
7. **Never weaken a check to make a change pass.** Deleting an assertion, adding
   an ignore attribute, loosening a matcher, or lowering a threshold to get a
   green result is the specific failure
   [ADR 0006](docs/adr/0006-the-gate-must-resist-what-it-gates.md) exists to stop.
   If a check fails, either fix the code or say plainly that you could not.

## Picking this project up

State lives in GitHub, not in a document someone has to remember to update:
**open issues** are the work, **milestones** are the stages, **releases** are
what shipped, and `docs/adr/` is why every decision is what it is. Start with
the issue list.

## Verify before you claim

The standard is that every claim in this repository is backed by something that
runs. It is the product thesis applied to the project's own documentation, and
it is the thing most likely to slip. It has slipped, and every time it was the
same mistake — shipping a claim stronger than the implementation:

| claimed | actual |
|---|---|
| "GitLocus reads `VOUCHED.td`" | no reader existed |
| "the release is immutable" | a repository setting, never enabled |
| the gate reported a verdict on "the evidence" | it could only see its own workflow |
| approvals were counted | from a self-asserted string |
| README announced v0.0.1 | v0.0.2 had shipped |
| a `Code of Conduct` reporting address | `gitlocus.dev` does not resolve; it would have bounced |
| `skipped` counted as a deterministic pass | invariant 3 violated in the product, not the config |
| crates named `locus-core` / `locus-cli` | both already taken on crates.io; unpublishable |
| a step named "Publish immutable release" | see row two |

Run it, read the output, quote it. Under-claiming costs nothing; over-claiming
costs the benefit of the doubt on everything else you say.

**The structural reason this keeps happening:** claims live in prose, and prose
has no CI. The durable fix is not to be more careful — it is to convert a claim
into something that runs. That is the product thesis applied to the repository
that ships it, and where a claim cannot be made executable, it should be written
as the weaker thing that is true.

## Traps worth not rediscovering

- **Local Rust:** use `cargo +stable-x86_64-pc-windows-gnullvm`. The default
  toolchain resolves to an MSVC target where MSYS `link` shadows MSVC's linker.
- **Windows CI runners default to PowerShell**, where `"$VAR"` silently expands
  to nothing. Set `shell: bash` as a job default, not per step.
- **`pull_request_target` is banned here** and `supply-chain.yml` enforces it.
  Several actions document that trigger as their normal usage; use
  `pull_request` and accept the reduced behaviour on fork contributions.
- **`gh attestation verify` yields signer identities** shaped exactly like a
  `signed_by` glob. Try that before building a signing path.
- **A permissive `signed_by` glob is close to no constraint** — anyone can run a
  workflow in their own fork and get a valid identity from the same issuer. Pin
  the workflow path.

## Before proposing a design change

Read [`docs/AGENT-ERA.md`](docs/AGENT-ERA.md). It records the problems this design
is an answer to — including swarms, harnesses, and capability that does not exist
yet — and names which problems are still unsolved. Proposals that re-solve a
settled problem, or that quietly reintroduce one, are the main way a project like
this drifts.

## Conventions

- Every source file starts with `// SPDX-License-Identifier: Apache-2.0`.
- Public items carry doc comments; `missing_docs` is a warning and CI denies warnings.
- Comments explain **why**, not what. If a comment restates the code, delete it.
- Tests are named as the claim they make — `assessed_evidence_never_satisfies_a_requirement`,
  not `test_policy_3`.
- Changes to `spec/` require matching changes to the conformance suite in
  `crates/gitlocus-core/tests/conformance.rs`. The policy enforces this.

## Where things are

| path | what |
|---|---|
| `spec/` | Normative model and JSON Schemas |
| `crates/gitlocus-core/` | Reference implementation of the model |
| `crates/gitlocus-cli/` | The `locus` binary |
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
