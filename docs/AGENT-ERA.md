<!-- SPDX-License-Identifier: Apache-2.0 -->
# What we are designing for

The problems this project expects, including the ones that have not arrived yet.

This document exists so the reasoning outlives the conversations it came from. A
design that is only defensible in someone's head drifts the moment that person is
busy.

It records *why*, not *status*. What is being worked on lives in the
[issue list](https://github.com/hey-vera/gitlocus/issues); the problems still
open are labelled [`open-question`](https://github.com/hey-vera/gitlocus/labels/open-question)
and are worth arguing with before anyone builds against them.

---

## 1. Swarms

Not one agent — a fleet. Two people running an orchestrator produce two hundred
agent instances in a day. Four things break:

**Identity inflates.** An identity per instance fills the trust graph with
single-use names that can never earn standing; one shared identity destroys
attribution. → [ADR 0007](adr/0007-actor-identity-is-a-delegation-chain.md):
standing attaches to the durable triple `(implementation, model, operator)`;
sessions are metadata.

**Sybil pressure.** Cheap identities, created freely. The design answer is
already in place and worth stating explicitly: `unknown` may *propose* anything
and *bind* nothing, so a thousand fresh identities buy exactly no leverage. This
property must survive every future change to tiers.

**Verdicts go stale under concurrency.** Fifty agents branch from one base; by
the time the thirty-seventh is evaluated the base has moved a dozen times.
Because `evaluate` is a pure function of (policy, contribution, evidence), a
verdict is **content-addressable**: hash the inputs, cache the output, recompute
freely. Purity was chosen for auditability and turns out to be what makes
swarm-scale evaluation affordable. Do not give it up.

**Cost — a design, not yet built.** Standing limits what a swarm can bind. It
does not limit what a swarm can *consume*: two hundred instances each opening a
contribution each triggering CI is resource exhaustion, and ranking makes the
queue survivable without making the compute free.

The shape that fits this architecture is **bring your own evidence**. Evidence is
already portable and signed, and evaluation is already separate from production —
so an untrusted contributor can run the checks themselves and submit the result,
and the project verifies a signature instead of spending its own compute. Cost
lands on the party creating it.

The trap is that a contributor running "the tests" can run *their* tests. The
answer is structural rather than contractual: the project publishes a **reusable
workflow**, the fork calls it, and the signing identity pins to the project's
workflow rather than the caller's. A caller cannot reach the reusable workflow's
OIDC token whatever it puts in its own YAML, so the identity attests that the
project's check definition ran.

This also explains why a permissive `signed_by` glob is close to worthless:
accepting any identity from a forge's issuer accepts a workflow in anybody's
fork, which is the situation the constraint exists to prevent.

Still open: priority and rate allocation for repositories that would rather spend
their own compute than require this. Tiered eligibility — repository CI runs
automatically above a tier, on request below it — is the likely shape, and it is
what forges already do informally for first-time contributors.

---

## 2. Harnesses

The strategically important observation, and the one most likely to be missed:

> **Provenance exists only at the moment of production, and the harness is the
> only thing present at that moment.**

The harness knows what files it read, what it tried, what it discarded, what it
ran, and which model produced which hunk. By the time a diff reaches a forge, all
of it is gone and unrecoverable. Every downstream tool is reduced to guessing
from the artifact — which is the losing game this project already refuses to play
([ADR 0002](adr/0002-no-ai-authorship-detection.md)).

Two consequences:

1. **The most valuable integration surface is the harness, not the forge.** A CI
   action collects evidence about a change; a harness integration collects
   evidence about how the change *came to be*. Those are different products and
   the second one is ours.
2. **Agents are evidence producers, not only subjects.** The natural transport is
   MCP, because that is already how agents reach systems, and the natural place to
   declare a repository's requirements to an agent is `AGENTS.md`, because that is
   already what agents read.

This raises the priority of an MCP surface relative to where the roadmap
originally placed it.

---

## 3. Models that do not exist yet

Designing for capability we cannot test means being explicit about which
assumptions are load-bearing.

**Holds regardless of capability.** The evidence classes. The distinction is not
about reliability but about reproducibility, injectability and liability, none of
which improve with a better model — see
[ADR 0005](adr/0005-evidence-classes-survive-better-models.md), which exists
precisely because this will be challenged repeatedly.

**Gets worse with capability.** Gate gaming. A more capable agent finds the cheap
path to a green check *faster*, and weakening a test is usually cheaper than
fixing the code. → [ADR 0006](adr/0006-the-gate-must-resist-what-it-gates.md).

**Degrades quietly with capability.** The authorship declaration. If a model can
produce a correct fifty-file change, the human's claim to have controlled the
expressive choices becomes thinner, and a single checkbox covering all fifty
files becomes a rubber stamp. Granularity is therefore a security property rather
than a UX preference — see [ADR 0008](adr/0008-authorship-is-declared-not-detected.md).

Three mechanisms, in increasing order of how much they help:

1. **Silence is not a claim.** Absent a declaration, the weakest claim applies —
   `generated`, uncopyrightable. Nobody accidentally acquires ownership by saying
   nothing, and asserting authorship becomes a deliberate act.
2. **Granularity scales with the strength of the claim.** Declaring `generated`
   across fifty files costs one statement, because it asserts nothing. Declaring
   `human` across fifty files requires fifty statements. The cost of a claim is
   proportional to what it asserts, which is the property that keeps it honest as
   models improve.
3. **The harness emits, the human upgrades.** A harness knows which hunks it
   produced, so it can emit `generated` per hunk by default; a person then raises
   specific hunks. This is the durable answer, and it is another reason the
   harness is the integration surface that matters (§2).

**Changes meaning, not validity.** Human attestation. Today it usually means "I
read this and it looks right". In a world where machines review better than
people it will mean "I accept responsibility for this". The class survives
because accountability, not inspection, was always what it encoded.

**The bottleneck keeps moving.** Production was the constraint; evaluation is the
constraint now; if machines eventually produce and verify reliably, the
constraint becomes *intent* — which changes should exist at all. That question is
permanently human, and a system built around evidence and policy is well placed
for it, because "what should exist" is what a policy is.

---

## 4. Other agent-era problems

| problem | ours? | mechanism |
|---|---|---|
| **Slopsquatting** — agents invent package names, attackers pre-register them | **yes** | deterministic: require a new dependency to have existed before the contribution and to meet a minimum age |
| **Prompt injection via repository content** — issues and READMEs are attacker-controlled text your agents read | **partly** | class separation bounds review ([ADR 0003](adr/0003-evidence-classes.md)); a hijacked agent can no longer manufacture its own approval ([ADR 0010](adr/0010-an-attestation-needs-someone-to-attest.md)), so the escalation path to *merge* is closed. An agent with write access can still do damage that has nothing to do with contributions, and that is not our layer |
| **Knowledge collapse** — `git blame` points at an agent and the reasoning was in a discarded chat log | **yes** | a declared, signed rationale bound to a revision. Not chain-of-thought: goal, alternatives rejected, constraint that forced the shape |
| **Cost of agent work** | **yes** | already the pricing axis: metered on work done, never per seat or per agent |
| **Trust bootstrapping** — how a new identity earns standing | **yes** | the ladder is now complete: permission → `maintainer`, merged history → `contributor`, `VOUCHED.td` → `vouched`, otherwise `unknown` ([ADR 0009](adr/0009-trust-is-earned-from-merged-history.md)). Nothing is stored; never derive from the git author field, which anyone can set |
| Merge conflicts at machine speed | **no** | a structural merge that silently produces wrong behaviour is worse than a loud text conflict. Stays a non-goal |

---

## 5. What developers complain about today

Worth keeping honest about which of these are ours to fix.

**Ours.** Green checks you cannot trust — flaky suites, results carried across a
force-push (already closed: evidence is bound to a revision digest). No
prioritisation when review volume exceeds review capacity. No way to answer "is
this change safe to accept". Per-seat pricing that punishes growth and is
incoherent when one human runs fifty agents. Opaque metered billing with no
relationship to value received.

**Not ours, and we should say so.** Hosting. CI compute. Issue tracking. Code
search. Line-based diffs — a real complaint, but semantic diffing is a
correctness minefield and belongs to whoever wants to own it.

---

## 6. The shape this implies

Not a replacement for a forge — see
[ADR 0001](adr/0001-evidence-not-a-forge.md). The claim is narrower and more
defensible:

> **For everything the agent era broke, GitLocus is the one thing you add.**

Today a serious team needs a review bot, a licence scanner, a provenance tool, a
triage system and a policy engine — five vendors answering one question badly.
They are one layer because they are one question: *what do I know about this
change, and what does my project require?*
