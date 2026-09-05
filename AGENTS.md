<!-- SPDX-License-Identifier: Apache-2.0 -->
# AGENTS.md

Instructions for coding agents working in this repository. Follows the
[AGENTS.md](https://agents.md) convention.

If you are an agent reading this: you are welcome here. This project exists
because agent-produced work is valuable and needs a better way to be evaluated,
not because it needs to be kept out. What follows is what the gate will check.

## What this project is

GitLocus decides whether a change may enter a trunk, and makes that decision
reproducible by anyone. Five types carry the model — Actor, Contribution,
Evidence, Policy, Verdict — and the normative definitions live in
[`spec/README.md`](spec/README.md). Read that before changing anything in
`crates/gitlocus-core`.

That engine is the kernel of a git platform and is being built first because it
is the half that cannot be commoditised — see
[ADR 0011](docs/adr/0011-the-kernel-of-a-git-platform.md) for where this is going
and what the ordering costs.

## Build and test

```bash
just check   # build, tests, lint, fmt — the edit-compile loop
just ci      # every check a runner can run without a base revision
```

Each recipe is named for a required status check, and the workflows invoke the
recipes rather than restating the commands, so a check cannot mean one thing
locally and another in CI. `just --list` shows them all.

Three of the fifteen required checks cannot run in full here, and
[`.github/required-checks.txt`](.github/required-checks.txt) records which and
why: `mutants` and `gate` need a base revision, which they take as an argument
defaulting to `origin/main`, and the two CodeQL analyses run only on GitHub.

`crates/repo-conformance` asserts that the mapping holds: that every required
check has a workflow job and a recipe, that no workflow restates a command the
justfile already defines, and that the paths, links, version strings and
decision records this document points at are real. It runs as part of `tests`.
The reason it exists is a paragraph further down — claims live in prose, and
prose has no CI.

`just` is the only tool beyond the Rust toolchain that `just check` needs;
`cargo-deny`, `cargo-mutants` and `zizmor` are needed by the recipes named after
them, and the justfile header says how to install each. On Windows under MSYS,
set `LOCUS_CARGO` — the header says why.

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
what shipped, and `docs/adr/` is why every decision is what it is.

`just brief` prints all of it — open issues by milestone, the latest release, the
last five merges, whether the service is answering, and whether the settings this
project depends on are still set. It reads GitHub live and is never committed, so
it cannot be stale. Start there, then read [LEARNINGS.md](LEARNINGS.md).
[ADR 0017](docs/adr/0017-the-foundation-layer-is-executable.md) says why that is
a command rather than a file.

## Verify before you claim

The standard is that every claim in this repository is backed by something that
runs. It is the product thesis applied to the project's own documentation, and it
is the thing most likely to slip. Run it, read the output, quote it.
Under-claiming costs nothing; over-claiming costs the benefit of the doubt on
everything else you say.

Sixteen times it has slipped, and every time in the same direction — a claim
shipped stronger than the implementation. [LEARNINGS.md](LEARNINGS.md) is the
list, together with the traps that cost this project real time, and each entry
names the check that would catch a recurrence or says plainly that nothing does.
Read it before you write a sentence about what this repository does; add to it
when you find the next one.

## Before proposing a design change

Read [`docs/adr/`](docs/adr/). Those records are where the problems this design
answers are written down — including swarms, harnesses, and capability that does
not exist yet — together with what each decision costs and which problems are
still open. Proposals that re-solve a settled problem, or that quietly
reintroduce one, are the main way a project like this drifts.

Open questions that are not yet decisions live in the
[issue list](https://github.com/hey-vera/gitlocus/issues) under the
`open-question` label, not in a document.

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
| `crates/locusd/` | The pure evaluator over HTTP (AGPL-3.0-only) |
| `crates/locus-ledger/` | The stateful half: principals, then grants and the record (AGPL-3.0-only, ADR 0021) |
| `.gitlocus/policy.yml` | The policy this repository runs on itself |
| `docs/adr/` | Why decisions were made |
| `justfile` | Every check, defined once; CI invokes these recipes |
| `.github/required-checks.txt` | Each required check, and how it is run |
| `crates/repo-conformance/` | This document, under test |
| `LEARNINGS.md` | What went wrong before, and what now catches it |

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
