<!-- SPDX-License-Identifier: Apache-2.0 -->
# Continuity

**Read this first if you are picking this project up.** It is the state of the
world, the decisions you may not quietly reverse, and the standard the work is
held to.

It is versioned with the code on purpose. A handoff written in a chat window is
stale the moment someone commits; this file is updated by the same pull request
that changes what it describes. **Keeping it current is part of the work, not
paperwork after it.**

---

## 1. What this project is, in one paragraph

GitLocus records **how a contribution was produced**, lets a repository state in
its own file **what evidence a change must carry**, and evaluates the two into a
reproducible **verdict**. It is not a git host and not an AI-code detector. It
runs on top of whatever forge you already use. The flagship application is
**licence integrity** — keeping a project's licence enforceable in a world where
uncopyrightable machine output accumulates in it — with contribution triage as
the second application of the same machinery.

If you cannot explain why those are the *same* machinery, read
[ADR 0008](adr/0008-authorship-is-declared-not-detected.md) before designing
anything.

## 2. State as of 2026-08-18

| | |
|---|---|
| repo | `hey-vera/gitlocus`, public, Apache-2.0 |
| latest | **v0.0.2** — three platforms, signed SLSA provenance, verified end to end |
| merged | 6 PRs · 61 tests · 65 files · main green on all four workflows |
| OpenSSF Scorecard | **7.1** |
| open PRs / issues | none |
| local checkout | `C:\Users\Josh\Desktop\GitHub\gitlocus` |

**v0.0.1 is a dead tag with no release.** Its Windows build failed and it was
deliberately *not* re-pointed: for a provenance project a tag must identify what
was actually built. Do not "tidy" it away.

**Shipped and working:** the five-type model; deterministic policy evaluation
shared byte-for-byte between CLI and CI; `locus contribution` (builds the
contribution document from git, so adoption does not require hand-writing JSON);
`VOUCHED.td` interoperation; signature *verification* with `signed_by` identity
constraints; `approvals_signed_by`; a complete trust ladder; and a gate that runs
this project's own policy against its own pull requests.

**Known not done, and honestly labelled as such in the docs:** no signing
*producer*, so `signed_by` correctly refuses everything today. No authorship
records. No `locusd`, no web UI, no MCP surface. Gate-gaming resistance is
specified in [ADR 0006](adr/0006-the-gate-must-resist-what-it-gates.md) and **not
implemented — it is a live hole**.

## 3. Invariants — breaking one is a bug, not a trade-off

`AGENTS.md` carries the working list. These are the ones with teeth:

1. **Assessed evidence never binds.** No threshold, no flag, no model.
   [ADR 0005](adr/0005-evidence-classes-survive-better-models.md) answers the
   "but models are good now" argument permanently — read it before making that
   argument.
2. **Evidence bound to another revision never counts.**
3. **`inconclusive` is unmet, never a pass.**
4. **Verdicts are pure functions.** No clock, no network, no ambient state, no
   dependence on evidence order. This is also what makes verdicts
   content-addressable and therefore cacheable at swarm scale.
5. **A signer is never read from input.** `Evidence::signer` is
   `skip_deserializing`. Undo that and every `signed_by` rule everywhere becomes
   decorative.
6. **No AI-authorship detection.** Declaration by a named party is a different
   thing and is the mechanism we do use.
7. **`locus-core` performs no I/O.** Anything touching a file, a clock or a
   subprocess belongs in the CLI or the server.

## 4. Settled — do not relitigate without new information

Ten ADRs in [`docs/adr/`](adr/). The strategic ones:

- **Not a forge.** GitHub shipped Agent HQ and Agentic Workflows into "run agents
  near the repo". We occupy the layer above, forge-agnostic. A native git surface
  is Stage 4 and *conditional*.
- **Not competing on detection.** Black Duck and FOSSA have two decades of
  corpus, and detection output is probabilistic, which under our own rules can
  never bind. We witness process instead.
- **Licence split.** Apache-2.0 stays on spec/core/CLI so the format can become a
  standard. The future `locusd` server goes **AGPL-3.0 in a separate repo**
  (`hey-vera/locusd`) — not FSL/BUSL, which is not OSI open source and would be
  self-refuting for a project defending open source.
- **Pricing.** Metered on work done. **Never per seat, never per agent.** Free
  forever for public repositories. This is the wedge against incumbent pricing,
  which is incoherent when one human runs fifty agents.
- **Audience is everyone** — solo developers through enterprises. Product-led,
  compliance-funded.

## 5. Open, and genuinely undecided

Do not treat these as settled just because they are written down.

- **Swarm cost.** Designed, not built: bring-your-own-evidence anchored to a
  project-controlled reusable workflow. See [`AGENT-ERA.md`](AGENT-ERA.md) §1.
- **Declaration granularity.** Designed, not built. Silence is not a claim; cost
  proportional to the strength of the claim; harness emits per hunk.
- **Prompt injection against agents with write access.** The escalation path to
  *merge* is closed ([ADR 0010](adr/0010-an-attestation-needs-someone-to-attest.md));
  the rest is not our layer and we should stop pretending otherwise.
- **Cross-repository trust.** Governance question, not a technical one.
- **In-toto predicate registration** — register upstream, or namespace under
  SLSA's `ORG_SOURCE_`?

## 6. The standard

Every claim in this repository must be backed by something that runs. That is
the product thesis applied to its own documentation, and it is the single thing
most likely to slip.

**It slipped four times already.** All four were the same mistake — shipping a
claim stronger than the implementation:

| claim | reality | how it was caught |
|---|---|---|
| "GitLocus reads `VOUCHED.td`" | no reader existed | audit before adding features |
| "the release is immutable" | a Settings toggle, never enabled | tried to verify it |
| gate reported a verdict on "the evidence" | it could only see its own workflow | ran it on a real PR |
| approvals were counted | from a self-asserted string | asked who actually signs |

If you find another, fix the claim *and* record it here. A list that stops
growing because nobody is looking is worse than no list.

**Working rules:**

- **Land work in reviewable slices.** Every PR green before merge; no long-lived
  branches. Six PRs so far, each independently coherent.
- **Comments explain *why*.** A comment restating the code should be deleted.
- **Name tests as the claim they make.** `assessed_evidence_never_satisfies_a_requirement`,
  not `test_policy_3`.
- **Negative tests are the point.** The valuable ones assert what must *not*
  happen: forged signer discarded, wrong identity rejected, agent cannot approve
  itself.
- **An ADR must state what the decision costs.** One with only benefits is an
  advertisement.
- **Verify before claiming.** Run it, read the output, quote it. "Tests pass"
  without running them is the failure this whole project is about.

## 7. Where to look

| | |
|---|---|
| [`AGENTS.md`](../AGENTS.md) | invariants, build commands, conventions — agents read this first |
| [`docs/adr/`](adr/) | why every decision is what it is |
| [`docs/AGENT-ERA.md`](AGENT-ERA.md) | the problems being designed for, including unsolved ones |
| [`docs/ROADMAP.md`](ROADMAP.md) | five stages; Stage 4 conditional |
| [`spec/README.md`](../spec/README.md) | the normative model; §8 is the open questions |
| [`THREAT-MODEL.md`](../THREAT-MODEL.md) | what is defended and what is not |
| [`.gitlocus/policy.yml`](../.gitlocus/policy.yml) | the policy this repo runs on itself |

## 8. Operational notes

- **Local Rust:** use `cargo +stable-x86_64-pc-windows-gnullvm`. The default
  toolchain resolves to an MSVC target where MSYS `link` shadows MSVC's linker.
- **Four gates before every push:** `build --all-targets`, `test`,
  `clippy --all-targets -- -D warnings`, `fmt --all --check`.
- **Bash heredocs mangle backticks** in this environment — write files with an
  editor tool, not `cat <<EOF`, when the content contains them.
- **Windows CI runners default to PowerShell.** `"$VAR"` silently expands to
  nothing. `release.yml` sets `shell: bash` as a job default; do the same
  anywhere else that matters.
- **`gh attestation verify` yields signer identities** of the form
  `https://github.com/OWNER/REPO/.github/workflows/FILE.yml@refs/tags/TAG` —
  exactly what a `signed_by` glob matches. **Try this before building a cosign
  path**; the producer we assumed we needed may already exist.
- **GitHub Actions billing on the `hey-vera` org is failing.** Public repos are
  unaffected, so GitLocus is fine, but private repos in the org cannot run CI.

## 9. Next

In order, with reasons rather than as a wish list:

1. **Signing producer.** Until it exists, `signed_by` is a rule nothing can
   satisfy. Try GitHub attestations first (§8).
2. **Gate-gaming resistance** ([ADR 0006](adr/0006-the-gate-must-resist-what-it-gates.md)).
   A live hole: a passing-tests record binds even when the tests changed in the
   same contribution. Honest evidence, false conclusion.
3. **Authorship records** ([ADR 0008](adr/0008-authorship-is-declared-not-detected.md)).
   The flagship capability. Nothing sells the project until this exists.
4. **Actor identity restructure** ([ADR 0007](adr/0007-actor-identity-is-a-delegation-chain.md)).
   Cheap now; the cost grows with every hour of trust history recorded under the
   flat shape.
5. **Composite Action.** v0.0.2 ships binaries, so adoption can be five lines.
6. **Stage 1** — `hey-vera/locusd`, OpenAPI-first, GitHub App, MCP surface.

Items 2 and 4 are cheaper today than they will ever be again. Item 3 is the one
that makes the project matter to anybody outside it.
