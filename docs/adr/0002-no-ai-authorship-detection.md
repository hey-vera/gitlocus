<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0002 — No AI authorship detection

- **Status:** accepted
- **Date:** 2026-08-18

## Context

The obvious product for a repository drowning in machine-generated pull requests
is a detector: classify each contribution as human or AI, and filter accordingly.
It is the first thing people ask this project for.

It does not work, for reasons that are structural rather than temporary:

- Final source text carries no reliable signal of its production process.
  Attribution markers are trivially removed.
- Model output is increasingly indistinguishable from human output, and the gap
  closes monthly. Any detector is dated on release.
- The category is incoherent at the edges. Code a human wrote with completion
  assistance, code a model wrote that a human rewrote, and code a human wrote from
  a model's explanation are all "AI-assisted" to different and unmeasurable
  degrees.
- The error mode is vicious. A detector punishes contributors who disclose and
  rewards those who conceal, which is precisely backwards.

## Decision

GitLocus will never attempt to classify a contribution by authorship.

It tracks two different things instead:

1. **Who is answerable** — the Actor, and specifically whether a named human has
   accepted responsibility. `ActorKind::Pair` exists for exactly this: an agent
   did the work, a person is accountable for it, and both are recorded.
2. **What can be proven** — deterministic Evidence, reproducible by a third party.

Neither depends on knowing how the code was typed.

## Consequences

**Good.** The system stays correct as models improve, because nothing in it
depends on a distinguishability that is eroding. It is honest with contributors:
disclosure carries no penalty, so there is no incentive to hide. And it targets
the property that actually matters — a change nobody will answer for is a problem
whether a human or a model produced it.

**Bad.** We cannot offer "block all AI contributions", which some maintainers
genuinely want. The honest answer is that we cannot deliver it and neither can
anyone else; what we can offer is a bar that unattributable work does not clear.

**Enforced, not just documented.** This is listed as a non-negotiable rule in
`AGENTS.md`, so an agent working on this codebase is told not to add one.

## Alternatives rejected

**Self-declared authorship as a required field.** Unverifiable, and it converts
the design into an honesty tax on honest people.

**Stylometry or perplexity scoring.** Would produce assessed evidence at best.
Under this project's own model, assessed evidence cannot bind — so the detector
could never gate anything, which makes building it pointless by our own rules.

## Related

The distinction here — provenance and accountability versus authorship attribution
— is the same one Linux kernel maintainers landed on: AI-assisted contributions
are acceptable provided a human takes responsibility and review happens.
