<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0020 — Identity is federated; standing and delegation are native

- **Status:** accepted
- **Date:** 2026-09-05
- **Amends:** [0007](0007-actor-identity-is-a-delegation-chain.md) — its alignment note said GitLocus records what a chain did and issues no credentials; this record has it issue exactly one, the delegation

## Context

Today the gate borrows GitHub's identity. `action.yml` derives a tier from
`repos/{repo}/collaborators/{user}/permission` and a merged-pull-request search,
and every actor's `key_binding` is GitHub's OIDC issuer.
[0009](0009-trust-is-earned-from-merged-history.md) called that "the stopgap that
makes the tier usable now" and named the evidence ledger as the durable answer.
[0007](0007-actor-identity-is-a-delegation-chain.md) said standing attaches to the
durable triple `(implementation, model, operator)`; #11, the change that gives the
`Actor` that shape, is unbuilt, and the type carries neither a `model` nor a chain.

[0019](0019-the-locus-is-identity-standing-and-the-decision.md) makes identity
native, which raises the question 0007 deferred: does GitLocus issue credentials,
or only validate them? 0007's alignment note, and the authorization model the MCP
specification assumes — a resource server validating tokens it did not issue —
both point at validating. For a human that is right, and it is what federated
login is. For an agent it is not enough, for a specific reason.

"Give my agent access to my account" is the feature the maintainer named, and it
has a security shape 0007 already fixed: **scope attenuation**. A delegated actor
may never hold a tier above the actor that delegated to it. No upstream credential
can carry that. A GitHub fine-grained token with write scope *is* the human for
every purpose the forge understands; there is no "may propose, never approve", no
tier ceiling, no set of permitted acts. The scope GitLocus needs is GitLocus's, so
the credential that carries it has to be GitLocus's. Everything else it can accept
from others.

There is a second shape to get right, and #74 supplies the principle.
"Answerable" is several requirements welded together, and what a contribution
must carry should scale with what it *claims*. Delegation is the same question
asked of an act rather than a contribution: what may a grant confer? A grant that
could confer an approval attestation reopens the exact hole
[0010](0010-an-attestation-needs-someone-to-attest.md) closed — an injected agent
manufacturing a human sign-off — except with a valid signature, so nothing looks
wrong. #74's discussion named the failure in advance: authority to sign is not
knowledge of what was signed.

## Decision

Five parts.

**1. A principal is GitLocus's durable identity for an answerable party.** It
binds one or more upstream identities — GitHub's OIDC first, GitLab and passkeys
later — and always holds at least one. Login is federated: GitLocus stores no
passwords and runs no reset flow. Standing (0009) attaches to the principal, so it
survives leaving any one upstream. The `operator` in 0007's triple is a principal.

**2. An agent has no identity of its own. It acts under a grant.** A grant is
issued by a principal to an agent `(implementation, model)`, scoped to a set of
repositories, capped by a tier ceiling, limited to a set of acts, bounded in time,
and revocable at any moment. Instances and sessions stay metadata (0007). Two
hundred instances under one grant share one standing and one answerable human,
which is correct.

**3. Attenuation is enforced in the evaluator, and it is pure.** The `Actor`
carries its chain: the tier the root principal holds in this repository, then each
grant's ceiling, root first. The effective tier is the minimum along the chain,
and a chain with no answerable party at its root is `unknown` regardless of what
any hop asserts. `gitlocus-core` computes this from the document alone; a property
test quantifies it over generated chains; the specification's conformance section
gains the clause. The stateful service constructs the chain and cannot make the
evaluator believe a higher tier than the chain permits, because the evaluator
recomputes it.

**4. What a grant can confer scales with what the act claims.**

| the act | it claims | delegable |
|---|---|---|
| propose a contribution | that work exists | yes |
| emit `deterministic` or `assessed` evidence | what ran, or an opinion | yes |
| declare authorship `generated` | nothing — no creative control | yes |
| declare authorship `human`, `directed_agent` or `derived` | a legal position | **no** |
| an approval attestation | a person accepted responsibility | **no** |
| issue, widen or revoke a grant | who may act for whom | **no** |

The non-delegable acts require the principal's own credential. Enforcement is at
the ingest boundary, before anything is recorded: a request bearing a delegated
credential that carries a non-delegable act is refused, and the test is named
`a_delegated_token_cannot_produce_an_attestation`. The evaluator is untouched by
this; it never sees a credential.

A repository that wants agents to merge unattended does not need a forged human.
It says `approvals: 0` on the paths and claims where that is its policy — which is
#77's vocabulary, and #77's decision to make per repository. Impersonation-proofing
is an invariant of the platform; waiving the human is a policy of a project.
Keeping those apart is what lets both be true.

**5. The credential.** Short-lived; signed by GitLocus's issuer key; carrying the
chain — principal, agent triple, ceiling, scope, grant identifier, expiry;
verifiable by any resource server holding the published key set; revoked by grant
identifier at the resource server. It is issued through the OAuth 2.1
authorization-code flow with PKCE that the MCP authorization specification
assumes, and the consent screen a harness sends its human to *is* the attenuation
interface: the human sees the agent, the repositories, the ceiling, the acts and
the expiry, and approves with their own session. GitLocus is the authorization
server for grants and the resource server for its own API. Nothing is invented: a
JWT with standard claims and one namespaced claim for the chain, in the namespace
the evidence predicate already uses; the exact names are decided where the issuer
is built.

## Consequences

**A store, and the one asset that cannot be reconstructed.** Principals,
bindings, grants, revocations and the ledger are state, kept beside the evaluator
and never in it ([0021](0021-state-lives-beside-the-evaluator-never-in-it.md)).
0007 said the cost of changing the actor's shape is proportional to the history
recorded under the old one. The ledger is about to start recording. #11 lands
first for exactly that reason.

**The issuer key is the crown jewel.** Whoever holds it mints delegations. It
lives on a host shared with other services, which is a real exposure and is
stated rather than hidden; rotation through the published key set is a day-one
requirement, not a later one.

**Login availability depends on GitHub until passkeys land.** Passkeys are
deferred until there is a second user to recover, because device loss and
recovery are the whole cost of native login, and there is no point paying it for
one person who can re-bind.

**The delegable split will be argued with every year.** "Let my agent approve the
trivial ones" is the request. The answer is in part 4: `approvals: 0` where the
project means it, never an agent wearing a human's signature.
[0005](0005-evidence-classes-survive-better-models.md)'s point about liability
applies unchanged — a model cannot bear it, and a delegated signature does not
transfer it.

**An agent driving its human's own session is indistinguishable from the human.**
A harness that operates the browser can approve with the principal's credential,
and this record does not prevent it. That is #15's rubber stamp with better
cryptography. Cost proportional to the claim — #15's second mechanism — is the
mitigation, and it stays open.

**`Actor` is a published type and changes shape.** `model` and a delegation chain
are added; the schema and conformance suite move with them. Existing documents
parse unchanged, and an empty chain evaluates as today.

**Standing still comes from the backend at first.** The ledger records what it
verified from day one, but a `contributor` tier derived from GitLocus's own
history needs history. Until then the tier at a chain's root is derived as it is
today, and 0009's stopgap has one more job before it is replaced.

## Alternatives rejected

**Native accounts with passwords.** The account-system tax — reset flows,
recovery, abuse — before there is a second user, and a reset email is an
account-takeover surface the project does not need. Federated first is the
adoption path applied to identity (0019).

**A pure resource server.** Validate upstream tokens and issue nothing. Cannot
express attenuation, cannot say "propose but never approve", and makes agent
access a feature of whichever forge issued the token. Forecloses the destination.

**Agents with their own keypairs that earn standing.** 0007 rejected identity per
instance, and #74 shows the adjudication problem unsolved. An agent with no
principal terminates at `unknown`, as 0007 said. The door to a bonded, non-human
answerable party stays open through #77's policy vocabulary, where a repository
can choose it — not through the identity model, where every repository would
inherit it.

**Delegable attestation behind a per-repository opt-in.** It makes the `attested`
class lie: the record would say a person accepted responsibility when a token did.
The honest expression of the same intent is `approvals: 0`.

**GitHub App installation tokens as the delegation.** Real and available today.
They carry no ceiling and no acts, and the delegation would exist only for
GitHub-hosted work, which is the dependency 0019 exists to remove.

**Waiting for the delegation standards to settle.** 0007 noted that no
production-ready standard traces a multi-hop chain back to a human in a way every
resource server can verify. Still true. Recording the chain in a JWT claim is the
smallest thing that works today and does not foreclose adopting a standard when
one exists: the chain is data, and data can be re-encoded.
