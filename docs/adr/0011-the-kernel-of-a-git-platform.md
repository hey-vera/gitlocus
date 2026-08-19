<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0011 — The kernel of a git platform

- **Status:** accepted
- **Date:** 2026-08-19
- **Supersedes:** [0001](0001-evidence-not-a-forge.md)

## Context

[ADR 0001](0001-evidence-not-a-forge.md) framed this project by what it would not
build. That framing was useful once — it stopped a founding brief from spending
its budget on git hosting — and it is now the wrong shape to build against,
because it describes a boundary instead of a destination.

The destination is a git platform: one place that converges what every existing
platform got right, for the two parties who are about to become one workflow — a
senior engineer accountable for a codebase, and the fleet of agents producing
most of the changes in it.

**What already works, and where it lives:**

| from | the good idea |
|---|---|
| GitHub | artifact attestations, rulesets, the pull request as the unit of work |
| GitLab | source, CI and policy in one place, CI defined as a file in the repository |
| Gerrit | change-centric review, evaluated per commit rather than per branch |
| sourcehut | patch series and email interop; contribution without an account |
| Radicle | sovereign, portable identity that does not belong to a host |
| SLSA · in-toto · Sigstore | signed, portable provenance with a defined envelope |
| Vouch | social trust as a plain file in the repository |
| Forgejo · Gitea | proof that a small team can build the commodity half |

None of these is scarce. Every one of them is scattered across a different
vendor, and each answers only part of one question.

**The question they all exist to answer.** Pull requests, reviews, required
checks, protected branches, merge queues, CODEOWNERS, rulesets — every one of
those features exists to decide *should this change enter the trunk, and on whose
authority?* Today that decision is an unportable pile of host-specific settings.
Only the host can compute it, nobody else can recompute it, and it does not
survive leaving the host.

## Decision

**Build the decision engine as the platform's kernel, and build it first.**

A Contribution keyed by content digest, Evidence classed by what it is worth, a
Policy versioned in the repository it governs, and a Verdict that is a pure
function of the three. That is a merge decision an auditor, a downstream
consumer, or the maintainer themselves can recompute offline from signed inputs.

Four capabilities follow from the kernel and from nothing else:

1. **A merge decision anyone can recompute** — offline, from signed inputs, on
   any host, years later.
2. **A licence that stays enforceable** as machine output accumulates, with a
   signed record behind every merged line ([0008](0008-authorship-is-declared-not-detected.md)).
3. **Governance you own** — versioned, diffable, reviewable, portable.
4. **Provenance captured where it exists** — at the harness, at the moment of
   production ([0012](0012-the-harness-is-the-integration-surface.md)).

Storage, smart HTTP, issues and UI are the commodity half and stay commodity;
Forgejo and Gitea are the evidence that they are buildable when they are needed.
The kernel is where the work goes, because it is the half that cannot be
commoditised.

Forge-agnosticism is a **platform feature**, not a constraint on one. GitLocus
can host and still evaluate a contribution that arrived as a GitHub pull request,
a GitLab merge request, or a mailed patch series, and give all three the same
verdict. That is the convergence: one place where work produced anywhere lands
and is decided the same way.

## Consequences

**Good.** The differentiated half ships first and is useful before any surface
exists around it. Adoptable one repository at a time with no migration, which
means the platform is eventually a surface added to something people already
depend on rather than a migration asked for up front. The kernel is small enough
to specify, and a specified kernel can have other implementations.

**Bad.** Until a surface exists, the UX ceiling is whatever a check run and a
comment can express. We depend on the forge to report changed paths honestly.
And "the merge decision, made properly" is a harder story to tell than "a new
GitHub", so the licence-integrity application carries the narrative until the
platform surface exists.

**The cost of the ordering.** Choosing the kernel first means years where the
platform claim is a direction rather than a product. That is a real cost and it
is accepted deliberately.

**This supersedes a boundary, not a judgement.** Everything 0001 said about *not
spending the budget on hosting first* remains correct and is preserved below as
a rejected alternative.

## Alternatives rejected

**Build the forge first, add the decision engine later.** Spends the entire
budget on the commodity half and produces a prettier GitHub. This was 0001's
central finding and it survives the reframing unchanged: the surfaces are
buildable later precisely because they are commodity, and the kernel is not.

**Sovereignty as the reason to migrate.** Radicle has pursued sovereign
peer-to-peer git since 2018 and the market has not moved. The lesson is that
owning your own infrastructure is not, by itself, worth a migration. The reason
has to be a decision you cannot get anywhere else — which is what this record
commits to building.

**Stay a plugin permanently.** Defensible, and it forecloses capabilities 1 and 3
above at the ceiling set by whatever the host chooses to expose. A verdict
rendered as somebody else's check run can never be the authoritative one.
