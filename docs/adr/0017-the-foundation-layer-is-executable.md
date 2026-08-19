<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0017 — The foundation layer is executable

- **Status:** accepted
- **Date:** 2026-08-19

## Context

[LEARNINGS.md](../../LEARNINGS.md) records sixteen occasions on which this
repository shipped a claim stronger than its implementation. Every one of them
was a sentence in a document, and until now no document in this repository had
CI. That gap was the last one: the code has 127 tests and zero surviving
mutants, the workflows are audited by zizmor and pinned to full SHAs, the
licence map is enforced by a check, the MSRV is compiled rather than asserted —
and the file explaining all of that was unchecked prose.

It was also wrong. AGENTS.md said of the four commands it listed:

> All four must pass. They are the same commands CI runs; there is no CI-only
> step.

CI passed `--locked` on all four and on none of the four as documented, ran
`cargo test --doc`, `cargo deny` and `cargo mutants` besides, and the four
covered four of the fifteen required status checks. A contribution could do
exactly what the documentation said and be red on eleven of them. Nothing had
noticed, because nothing could.

The immediate question was whether the foundation — AGENTS.md and the documents
around it — was structured well, and whether it should be split into more files:
a memory file, a learnings file, a decisions file. That framing assumes the
problem is organisation. It is not. The documents were reasonably organised and
substantively good. What they lacked was any mechanism by which being wrong
costs something.

## Decision

**The foundation layer is executable: every claim it makes that can be checked,
is checked, and the claims that cannot be are written as the weaker thing that
is true.**

Three parts.

**1. One definition of every check.** `justfile` defines each check once, with a
recipe named for the status-check context it satisfies, and the workflows invoke
those recipes rather than restating the commands.
`.github/required-checks.txt` records the mapping — including which checks
cannot run in full outside GitHub and why. The claim "there is no CI-only step"
is now kept true by being the thing CI runs, rather than by anyone remembering.

**2. `crates/repo-conformance` puts the documents under test.** Eight tests,
each named as the claim it makes: relative links resolve, the where-things-are
table points at things that exist, every required check has a job and a recipe,
every decision record states what it costs, supersessions link both ways, every
record is in the index, one version string means one version, and every learning
names its guard.

**3. `LEARNINGS.md` carries the record, and every entry names its guard.** The
guard is a test, a path, a recipe, an issue, or an explicit `none` with a reason.
Three entries are about GitHub settings that no test in the tree can reach;
`just brief` guards those, and is the only piece that needs a token.

## Consequences

**A `just` dependency.** One pinned action step in CI, and a tool a contributor
installs. That is a real cost paid for a real property, and if `just` ever
becomes an obstacle the recipes are shell and move back easily.

**Adding a required check now means three places** — the ruleset, a workflow job
and `.github/required-checks.txt` — and `every_required_check_has_a_job_and_a_recipe`
fails until all three agree. That friction is the guarantee. It is also the most
likely reason someone will want to weaken this test, which is exactly the move
[ADR 0006](0006-the-gate-must-resist-what-it-gates.md) exists to stop.

**The conformance suite will block a legitimate documentation change.** Renaming
a decision record, moving a file, or bumping the version touches more than one
place, and the test will say so. That is the intended behaviour and it will still
be annoying.

**`.github/required-checks.txt` can drift from the ruleset, and only `just brief`
catches it.** `brief` needs a token, so it cannot be a required check, and
nothing runs it automatically. The file is checked in precisely so that the
offline half can exist at all; the split is stated in both places rather than
hidden.

**The suite encodes today's true rules, not tidier ones.**
`every_adr_states_what_it_costs` accepts two spellings of the cost section
because 0005 and 0007 use a different one and are better for it. That is an
exception in a test, and exceptions accumulate. The alternative — editing good
records so a test can be simpler — is the failure this project is about, in
miniature.

**A guard can be checked for existence, not for quality.** `every_learning_has_a_guard`
asserts that a named test exists. It cannot assert that the test would actually
catch the thing. Three traps in LEARNINGS.md say `none` because nothing guards
them, and that honesty is load-bearing: the moment a `none` starts feeling
embarrassing enough to fill in with something plausible, this whole mechanism
becomes decoration.

## Alternatives rejected

**A `DECISIONS.md`.** [`docs/adr/`](README.md) already is that, and better: one
record per decision, so supersession is visible in the status line and in the
filename, and two people editing different decisions do not conflict. A single
file makes 0001-superseded-by-0011 invisible unless you read the whole thing.

**An in-repo `MEMORY.md`.** A status document by another name. AGENTS.md says
state lives in GitHub — open issues are the work, milestones are the stages,
releases are what shipped — precisely because a checked-in status file rots
silently and no test can tell. `just brief` is the non-rotting form of the same
idea: it reads the state live, prints it, and is never committed, so it cannot
be stale. What it costs is a token and a network call, which is why the offline
half of the same guarantee lives in the conformance suite instead.

**Correcting the false sentence and moving on.** This is what was done the
previous sixteen times, and LEARNINGS.md is what that produces.

**An `xtask` crate instead of `just`.** No installation, which is genuinely
better. Rejected because it would put the definition of `cargo deny`, `zizmor`
and the DCO shell check inside a Rust binary that must compile before any check
can run — including on the MSRV job and the Windows leg, which is exactly where
the toolchain traps live. A task runner that can fail to build is a worse task
runner than one that needs installing.

**Generating the command block in AGENTS.md from the workflows.** It would make
drift impossible rather than merely detectable. Rejected because it makes the
document a build artifact, and a document nobody may edit is a document nobody
reads. Detecting drift and failing loudly keeps the prose human.

**A merge queue.** Considered because the `main` ruleset requires branches to be
up to date, so a second pull request needs a manual rebase. Rejected: with one
maintainer landing one contribution at a time, a merge queue adds a full CI run
of latency to every merge and solves a contention problem this repository does
not have. Worth revisiting the first time two contributions are genuinely in
flight at once.

**A push ruleset restricting `.github/workflows/**` and `.gitlocus/**`.** This
would make real the restriction AGENTS.md describes, and close a LEARNINGS.md row
directly. Rejected because `.gitlocus/policy.yml` already expresses it as
`min_tier: maintainer` on those paths — the product's own mechanism, evaluated by
the product's own gate. A repository that reaches for the forge's feature instead
of the thing it is building has stopped being its own first adopter, which is the
only reason anyone should believe the gate works.

**A `.claude/settings.json`, or any other assistant-specific configuration.**
Rejected to keep the foundation vendor-neutral. `CLAUDE.md` exists in this
repository only as a one-line import of `AGENTS.md`, so that there is exactly one
document and no drift between tools; adding tool-specific configuration would
undo the reason that file is shaped the way it is. What a cold session actually
needs — where the project is, what is required, whether the service is up — is
`just brief`, which every tool can run.
