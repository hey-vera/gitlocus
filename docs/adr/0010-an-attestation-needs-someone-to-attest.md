<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0010 — An attestation needs someone to attest

- **Status:** accepted
- **Date:** 2026-08-18

## Context

`attested` evidence is documented as "a human took responsibility" and counted as
an approval. The counting used `produced_by` — a free-text string chosen by
whoever wrote the record.

So an approval was a document asserting that somebody approved. Nothing checked
that anybody had.

That is the same category of mistake as reading `signer` from input, which
[the signing work](0005-evidence-classes-survive-better-models.md) had just
closed, and it is worse in consequence. It is also the concrete form of
the problem this design has always treated as unsolved: **prompt injection against
an agent with write access.**

The attack needs no sophistication. An agent reads repository content — an issue
body, a README, a dependency's documentation, a pull request description — all of
which are written by parties the project does not trust. The content contains
instructions. The agent, now working for someone else, opens a contribution and
emits an `attested` record with `produced_by: "a-maintainer"`. The policy counts
an approval. Class separation does not help: the record genuinely is `attested`.
It is simply nobody's attestation.

## Decision

**A policy may require that an attestation carry a verified signature from a
named identity before it counts as an approval.**

```yaml
require:
  approvals: 1
  approvals_signed_by: "https://github.com/login/oauth/*"
```

Two consequences follow from the implementation:

**Approvals are counted by signer, not by claimed producer, whenever the
constraint is set.** Otherwise one party could satisfy a two-approval rule by
varying a string.

**Where several matching rules constrain approvals, all must match** — consistent
with how signature constraints on checks already behave.

The constraint is opt-in and absent by default, so existing policies do not
change meaning under their feet. That is a deliberate compromise and its cost is
stated below.

## Consequences

**The blast radius of a hijacked agent becomes bounded.** With this set, an
injected agent can still open a contribution — we do not prevent that and it is
not our layer — but it cannot manufacture the human sign-off that would merge
one. The failure mode moves from "an unwanted contribution was merged" to "an
unwanted contribution was opened", which is a nuisance rather than an incident.

**Prompt injection is still not solved, and this record does not claim to solve
it.** An agent with write access to a repository can do damage that has nothing
to do with contributions. What is closed here is one specific escalation path.

**Opt-in means most policies remain exposed.** A safer default would be to
require a signer for every approval, and it is the right long-term shape. It is
not the default today because no producer tooling exists yet, so defaulting to
required would mean no policy in existence could be satisfied. When signing
producers land, the default should be revisited — and this paragraph exists so
that revisiting is a decision someone makes rather than something everyone
forgets.

**Human identity providers must be named carefully.** The same hazard applies as
to check signatures: a glob matching only an issuer accepts anyone that issuer
will sign for. Pinning matters.

## Alternatives rejected

**Infer humanity from the actor's kind.** The evidence record does not carry the
producing actor's kind, and adding it would not help — it would be one more
self-asserted field, which is exactly the problem being fixed.

**Forbid agents from emitting attested evidence.** Unenforceable without knowing
what produced a record, which is the thing a signature establishes and nothing
else does.

**Treat forge review approvals as the only source.** Correct for GitHub and
useless everywhere else. The specification is forge-agnostic; a GitHub review is
one way to produce a signed human attestation, not the definition of one.
