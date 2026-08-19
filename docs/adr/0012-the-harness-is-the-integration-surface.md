<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0012 — The harness is the integration surface

- **Status:** accepted
- **Date:** 2026-08-19

## Context

The roadmap treated the forge as the place to integrate: a check run, a comment,
a pull request decorated with a verdict. That is where the *decision* is
consumed, and it is the wrong place to collect what the decision is made from.

> **Provenance exists only at the moment of production, and the harness is the
> only thing present at that moment.**

An agent harness knows which files it read, what it tried, what it discarded,
what it ran, and which model produced which hunk. None of that survives into a
diff. By the time a change reaches a forge, every fact about how it came to be
has been destroyed, and any downstream tool is reduced to guessing from the
artifact — the losing game already refused in
[0002](0002-no-ai-authorship-detection.md).

This is also what makes the hardest open problem in
[0008](0008-authorship-is-declared-not-detected.md) tractable. Declaration
granularity is a security property: one checkbox covering fifty files is a rubber
stamp, and it gets worse as models improve. A human cannot realistically declare
authorship hunk by hunk. A harness can, because it already knows.

## Decision

**Treat the harness as a first-class evidence producer, and build for it at the
same priority as the forge integration.**

Three consequences:

1. **Agents are evidence producers, not only subjects.** The same agent whose
   work is being evaluated is the best-placed party to record how that work was
   produced. This is not a conflict of interest as long as what it produces is
   classed honestly — a harness emits `assessed` and `attested` records that bind
   nothing on their own, and the class system already handles the rest
   ([0003](0003-evidence-classes.md)).

2. **MCP is the transport.** It is already how agents reach systems, so a
   provenance surface delivered as MCP tools requires no new integration path in
   any harness that already speaks it.

3. **`AGENTS.md` is where a repository states its requirements to an agent.** It
   is already what agents read. A project's policy should be legible there rather
   than only in a file the agent has no reason to open.

**The declaration ladder this unlocks**, in increasing order of how much it
helps:

- **Silence is not a claim.** Absent a declaration, the weakest applies —
  `generated`, uncopyrightable. Nobody acquires ownership by saying nothing, and
  asserting authorship becomes a deliberate act.
- **Cost proportional to the claim.** `generated` across fifty files is one
  statement because it asserts nothing; `human` across fifty files is fifty
  statements. This is the property that keeps declarations honest as capability
  rises.
- **The harness emits, the human upgrades.** The harness emits `generated` per
  hunk by default and a named person raises specific ones. This is the durable
  answer to granularity, and it is only available at the harness.

## Consequences

**Good.** It is the one integration surface no forge occupies, and it collects
facts that are unrecoverable anywhere else. It makes 0008's granularity problem
solvable rather than merely acknowledged. And it aligns with where the kernel is
going: evidence produced anywhere, evaluated the same way
([0011](0011-the-kernel-of-a-git-platform.md)).

**Bad.** There are many harnesses and no standard for what one exposes, so early
integrations are bespoke and some are impossible. A harness-produced record is
self-asserted until signed, which means the signing producer becomes a
dependency rather than a nicety.

**Cost.** Priority moves. An MCP surface is now Stage 1 work alongside the server
rather than something that arrives once the forge integration is finished.

**What it does not license.** Provenance means structured, verifiable facts about
what ran and what passed. It does not mean chain-of-thought, prompts, or model
parameters — none of which are verifiable, and treating them as evidence would
confuse narrative with proof.

## Alternatives rejected

**Forge-only integration.** Simpler and reaches users faster, but it can only
ever observe artifacts. It cannot answer the authorship question at all, which is
the flagship application.

**Reconstruct provenance from the diff.** This is authorship detection wearing a
different hat, and it fails for the reasons already recorded in
[0002](0002-no-ai-authorship-detection.md).

**Wait for a harness standard to emerge.** The facts are being destroyed now, in
every repository, and a standard nobody has implemented against is written badly.
