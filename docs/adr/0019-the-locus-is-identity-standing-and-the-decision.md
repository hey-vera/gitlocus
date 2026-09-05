<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0019 — The locus is identity, standing and the decision; storage is a backend

- **Status:** accepted
- **Date:** 2026-09-05
- **Amends:** [0011](0011-the-kernel-of-a-git-platform.md)

## Context

[ADR 0011](0011-the-kernel-of-a-git-platform.md) named the destination — a git
platform — and chose the merge decision as its kernel because hosting is the
commoditised half. It left the destination underspecified in one respect, and the
gap has started to cost.

Stage 3's done condition reads: *a project runs its merge decisions here without
moving its git storage.* The maintainer's stated goal is to use GitLocus instead
of GitHub entirely. Read one way those agree; read another they contradict, and
six months of work could assume either without anyone recording which. The
repository's own description on GitHub still says "Not a forge", which is
[0001](0001-evidence-not-a-forge.md)'s framing and was superseded by 0011.

The first adoption by another repository sharpened the question. `1xmint/notelocus`
ran the gate and it worked — and what it tested was GitLocus as a GitHub Action.
GitHub's identity supplied the tier, GitHub's check runs supplied the evidence,
GitHub's pull request displayed the verdict. It could not test GitLocus as a
platform, because there was nothing to log in to, no identity of GitLocus's own,
no way to hand an agent scoped access, and no place a maintainer could look. A
verdict rendered inside somebody else's product is, as 0011 already said, never
the authoritative one.

So this record answers what "instead of GitHub" means, precisely enough to order
the work.

**What makes a platform the thing you cannot leave is not the bytes.** Git is
distributed; `git remote set-url` moves a repository in one command, and every
host offers an import. What does not move is everything else: who you are there
(identity), what you have earned there (standing), who may do what on whose
authority (the decision, and the delegations behind it), and what needs your
attention (the queue). Those are the unportable pile of host-specific state 0011
described, and they are the reason a project stays. Every one of them is something
this record's predecessors already claim or design: the decision is the kernel
(0011), standing is derived from merged history and belongs to a durable triple
([0007](0007-actor-identity-is-a-delegation-chain.md),
[0009](0009-trust-is-earned-from-merged-history.md)), and the queue is Stage 2.

## Decision

**GitLocus is a git platform whose native surface is identity, standing,
delegation, the decision and the queue. Git storage is a pluggable backend.
GitHub is the first backend, because that is where the work is.**

Three clarifications follow, and each changes something on the record:

1. **Stage 3's "without moving its git storage" describes the adoption path, not
   the ceiling.** A project adopts GitLocus for its decisions while its bytes stay
   where they are; that is what makes adoption cheap. It is not a promise that the
   bytes stay there forever. The milestone text now says so.

2. **Stage 4 exists, conditionally: the authoritative merge.** On a backend,
   GitLocus's verdict is a check run the host can be configured to ignore. The
   stage opens when the first project wants a verdict that is authoritative — a
   ref that cannot move without a satisfied verdict — rather than advisory. Its
   likely first slice is a merge relay that owns the write path to a backend
   without owning the bytes; native storage comes when a project whose identity,
   standing and decisions already live here asks for it. 0001 deferred a native
   surface on the condition that storage unlock something the API layer cannot.
   This names the something.

3. **Identity, delegation and the ledger move ahead of ingest.** Stage 1 becomes
   them, and the records that follow this one (0020, 0021, 0022) say how. The
   GitHub App joins the GitLab and mail producers in Stage 3 as the GitHub
   backend connector. The next adoption test must exercise a login, a grant and a
   queue, or it tests the action again.

0011's central finding is untouched: the kernel came first, and it was right to.
What this record adds is that the platform above it is not "the commodity half
plus a verdict". It is the four things a host holds hostage, built natively, with
storage underneath them as the one part anyone can supply.

## Consequences

**Identity is state.** Verdicts are pure and stay pure; the evaluator has been
kept free of a clock, a network and a store, and that is what makes a verdict
content-addressable. A principal registry, grants and a ledger are exactly the
state it has been kept free of. Where that state lives, and how it stays out of
the evaluator, is an architectural question rather than a detail, and it gets its
own record (0021).

**A token issuer becomes the most security-sensitive component this project
has.** Whoever holds the issuer key can mint a delegation. The release workflow
held that title until now.

**Login depends on other people's providers until it does not.** Federated login
(0020) is the adoption path applied to identity. It means availability depends on
an upstream until a native credential exists, and "log in with GitHub" on a
GitHub replacement reads oddly. It is correct for the same reason Stage 3 is:
value before migration.

**The AGPL side grows** from a two-hundred-line evaluator to a service with a
store, a key and a login. Contributors meet the licence split
([0016](0016-locusd-lives-here-under-agpl.md)) more often.

**The years get longer.** 0011 accepted years in which the platform claim is a
direction rather than a product. Putting identity and delegation ahead of ingest
defers Stage 3; making Stage 4 conditional defers hosting indefinitely. That is a
cost to the destination's timeline, accepted so that what ships first is the part
no backend can offer.

**A setting was wrong, and settings are where the record cannot reach.** The
repository description said "Not a forge" for seventeen days after 0011. It is
corrected, and `just brief` — which guards the settings the record depends on —
now fails if it ever says so again. [LEARNINGS.md](../../LEARNINGS.md) has the row.

## Alternatives rejected

**Host first.** 0001 found that building the storage half first spends the budget
on the commodity half, and 0011 kept the finding. Nothing has changed it. A solo
maintainer building Forgejo before the decision engine has a surface anyone uses
is still the wrong order.

**Stay a surface on other platforms.** Never host, never issue identity; validate
other platforms' tokens. It is the cheapest reading and it forecloses the feature
the destination needs most: an agent delegation with a tier ceiling, which no
upstream provider can express and which therefore someone has to issue. It also
makes GitLocus a GitHub feature forever, which contradicts the goal outright.

**Replace GitHub by reimplementing it.** Issues, discussions, a file browser, code
review, stars. That is the prettier GitHub 0001 warned against, and it forgets why
GitHub's surface is enormous: twenty years of accretion. The decision is the
product, and the surface follows from that (0022).

**Sovereignty as the reason to move.** 0011's Radicle finding still holds. The
reason to move has to be something you cannot get anywhere else: a decision
anyone can recompute, an identity that survives leaving a host, and a delegation
that cannot exceed the human behind it.
