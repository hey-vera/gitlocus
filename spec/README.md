<!-- SPDX-License-Identifier: Apache-2.0 -->
# GitLocus Contribution Evidence — v0 (draft)

**Status: draft. Unstable. Will change without notice until v1.**

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are to be interpreted as
described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## 1. Scope

This document specifies a format for making verifiable claims about a proposed
change to a source repository, and a policy language for deciding what claims a
given change must carry before a human is asked to look at it.

It does **not** specify transport, storage, a signing scheme, or a user interface.
It is intended to compose with:

- [in-toto Attestation Framework](https://github.com/in-toto/attestation) for the
  statement envelope,
- [Sigstore](https://www.sigstore.dev/) for signing and transparency,
- [SLSA Source Track v1.2](https://slsa.dev/spec/v1.2/source-requirements), whose
  Source Provenance Attestations deliberately leave the evidence format to the
  source-control system. This specification is one such format.

## 2. Why this exists

SLSA answers *was this artifact built the way it claims*. This specification
answers the adjacent, currently unanswered question: *was this change **proposed**
the way it claims, and by whom*.

At the time of writing there is no registered in-toto predicate for contribution
or agent decisions; see [in-toto/attestation#554](https://github.com/in-toto/attestation/issues/554).

## 3. Core objects

### 3.1 Actor

An Actor identifies the origin of a Contribution.

An Actor MUST declare exactly one kind:

| kind | meaning |
|---|---|
| `human` | A person acting directly. |
| `agent` | An autonomous agent with no identified operator. |
| `pair` | An agent acting under an identified operator who accepts responsibility. |

An Actor of kind `agent` MUST NOT be treated as having a responsible human. A
verifier SHOULD hold such actors to the lowest trust tier regardless of any other
signal, because there is no party to hold accountable for the change.

An Actor SHOULD carry a `key_binding` naming the OIDC subject or public key its
signatures verify against. An Actor without a `key_binding` is unauthenticated:
its claims are self-asserted and a verifier MUST NOT grant them cryptographic
weight.

Trust tiers are ordered: `unknown` < `vouched` < `contributor` < `maintainer`.
A requirement for tier *T* is satisfied by any tier ≥ *T*.

Implementations SHOULD populate `vouched` from an existing
[`VOUCHED.td`](https://github.com/mitchellh/vouch) file where one is present,
rather than defining a competing trust file.

### 3.2 Contribution

A Contribution is identified by the triple
`(repository, base_digest, head_digest)`.

It MUST NOT be identified by a forge-assigned number. The same change observed as
a GitHub pull request, a GitLab merge request and a mailed patch series is one
Contribution. `forge_ref` MAY record where it was observed and is informational
only; a verifier MUST NOT make a decision from it.

### 3.3 Evidence

An Evidence record is a claim about one revision of one Contribution.

Every Evidence record MUST carry a `class`:

| class | definition | may satisfy a requirement |
|---|---|---|
| `deterministic` | Reproducible by any party with the same inputs. | **Yes** |
| `assessed` | Produced by a heuristic or a model. | **No** |
| `attested` | A human accepted responsibility. | Approvals only |

This separation is normative and is the central requirement of this
specification. An implementation that permits `assessed` evidence to satisfy a
blocking requirement is **not conformant**, irrespective of how it presents that
evidence to a user.

An Evidence record MUST carry `subject_digest`. Evidence whose `subject_digest`
does not equal the Contribution's `head_digest` MUST NOT satisfy a requirement.
This is what prevents a green result from before a force-push being counted
against the code that replaced it.

`outcome` MUST be one of `pass`, `fail`, `inconclusive`. An `inconclusive`
outcome MUST be treated as unmet. It MUST NOT be treated as a pass.

Evidence records SHOULD be transported as in-toto Statements with
`predicateType: https://gitlocus.dev/contribution-evidence/v0`, the Contribution's
`head_digest` as the statement subject, and the Evidence record as the predicate.

### 3.4 Policy

A Policy is a versioned document stored **in the repository it governs**. A
verifier MUST evaluate the Policy at the revision under evaluation, not a
Policy fetched from elsewhere.

Every rule whose `when.paths` matches at least one changed path contributes to
the outcome. When several rules match:

- required deterministic check names MUST be unioned,
- `approvals` MUST take the maximum demanded,
- `min_tier` MUST take the strictest demanded.

A change touching both ordinary source and CI configuration is therefore held to
the CI rule. Any other combination lets a contributor weaken the rule that governs
them by also touching a file governed by a weaker one.

### 3.5 Verdict

A Verdict MUST be a pure function of (Policy, Contribution, Evidence). The same
three inputs MUST always produce the same Verdict. No clock, network call,
ordering of the evidence array, or ambient state may affect it.

`decision` MUST be:

| decision | condition |
|---|---|
| `blocked` | The actor's tier is insufficient, **or** any required check is unmet. |
| `needs_human` | All required checks satisfied; attestations outstanding. |
| `satisfied` | Everything the policy demands is present. |

Tier insufficiency and unmet checks both produce `blocked` because neither
warrants human attention yet — which is the entire point.

## 4. Ranking

A Verdict carries ranking signals so a queue can be ordered by what a maintainer
can actually act on:

- `confidence`: satisfied required checks ÷ total required checks, or `1.0` when
  nothing is required.
- `human_cost`: outstanding attestations.

A queue SHOULD sort `satisfied` before `needs_human` before `blocked`. Blocked
work sorts last because it is the work a maintainer cannot move, and surfacing it
above work that is ready is the exact failure this specification exists to
address.

Ranking is advisory. A verifier MUST NOT let ranking alter a `decision`.

## 5. Reputation

This specification defines no reputation score and no reputation store.

Standing is **derived** from evidence history at evaluation time. A persisted
score becomes a target to optimise against, and the cost of generating evidence
that moves such a score is exactly the cost this specification assumes has already
collapsed.

## 6. Conformance

An implementation is conformant if it:

1. rejects `assessed` evidence as a means of satisfying a requirement;
2. rejects evidence whose `subject_digest` does not match the revision;
3. treats `inconclusive` as unmet;
4. unions requirements and takes the strictest approvals and tier across matching rules;
5. produces byte-identical verdicts for identical inputs;
6. evaluates the policy at the revision under evaluation.

The reference implementation lives in [`crates/locus-core`](../crates/locus-core);
the test suite there is the executable form of this section.

## 7. Schemas

- [`schemas/contribution.schema.json`](schemas/contribution.schema.json)
- [`schemas/evidence.schema.json`](schemas/evidence.schema.json)
- [`schemas/policy.schema.json`](schemas/policy.schema.json)
- [`schemas/verdict.schema.json`](schemas/verdict.schema.json)

## 8. Open questions

Recorded rather than hidden. These are unresolved and feedback is wanted:

1. **Signing.** v0 defines the payload, not the envelope. Sigstore keyless is the
   likely default, but agents in CI and agents on a laptop have very different
   key stories.
2. **Blast radius.** Path globs are a crude proxy for risk. Whether something
   better can stay deterministic is unproven.
3. **Evidence expiry.** Evidence is bound to a digest, but a passing test from six
   months ago against an unchanged file may still be stale in ways digests miss.
4. **Cross-repository trust.** Whether a tier earned in one repository should mean
   anything in another, and who would be entitled to say so.
5. **Predicate registration.** Whether this becomes an in-toto registered
   predicate or stays a `SOURCE_`-namespaced extension under SLSA.
