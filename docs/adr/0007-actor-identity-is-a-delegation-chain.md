<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0007 — Actor identity is a delegation chain, not a name

- **Status:** accepted
- **Date:** 2026-08-18
- **Amended by:** [0020](0020-identity-is-federated-standing-and-delegation-are-native.md) — GitLocus issues exactly one credential, the delegation; the chain lives on the `Actor` and is attenuated in the evaluator

## Context

The model currently gives an actor a flat `id: String`. That holds for one human
or one agent. It does not hold for a swarm.

A team of two people running an orchestrated fleet produces two hundred agent
instances in a day. Two ways to represent that, both wrong:

- **One identity per instance.** The trust graph fills with single-use identities
  that can never accumulate standing, and reputation becomes noise.
- **One shared identity for all instances.** Attribution collapses; nobody can
  say which instance did what, or narrow permission for one task.

The real structure is a **delegation chain**: a human authorises an orchestrator,
which spawns sub-agents, which call tools. The 2026 identity work names this
directly — multi-hop delegation — and reports that no production-ready standard
traces such a chain back to the originating human principal in a way every
resource server along it can verify. The governing principle from that work is
**scope attenuation**: each delegation step must narrow, never widen, the set of
permitted actions.

Standing belongs to the durable part of that chain. A session does not earn
reputation any more than a terminal window does.

## Decision

**Standing attaches to the durable triple `(implementation, model, operator)`.
Instance and session identifiers are metadata, never identity.**

- `implementation` — the harness (`claude-code`, `codex`, an in-house runner).
- `model` — what actually produced the work, when known. Two harnesses running
  different models are not interchangeable and should not share standing.
- `operator` — the human who is answerable. Already load-bearing in
  `ActorKind::Pair`; this record makes it the anchor of the whole chain.

**Scope attenuation is a rule, not a hope.** A delegated actor may never hold a
tier above the actor that delegated to it. An orchestrator at `contributor`
cannot spawn a sub-agent at `maintainer`. Enforced in evaluation, with a test.

**A chain with no human at its root terminates at `unknown`.** This already
follows from `ActorKind::Agent` having no responsible human; the chain framing
makes it general — attenuation from an unaccountable root attenuates from
nothing.

## Consequences

**Two hundred instances share one standing, and that is correct.** The operator
is answerable for all two hundred. Spinning up more instances buys no additional
trust, which is exactly the property that makes swarms safe to allow: cheap
identities are free to create and worth nothing, so there is no Sybil advantage.

**Reputation stays meaningful under swarm volume.** It accrues to a triple that
persists across sessions, so it reflects a track record rather than a burst.

**Attribution survives.** The session identifier is still recorded on the
evidence, so "which instance produced this" remains answerable — it simply is not
what trust is computed from.

**Cost is the remaining unsolved half.** Standing limits what a swarm can *bind*;
it does not limit what a swarm can *consume*. Two hundred instances each opening
a contribution each triggering CI is a resource-exhaustion problem this record
does not address. Rate and priority tied to tier is the likely answer, and it is
not designed yet.

**This changes a published type.** `ActorKind` gains fields; the schema and
conformance suite change with it. Doing it now costs an afternoon. Doing it after
the trust graph has history in it costs a migration of the one asset that cannot
be reconstructed.

## Alignment

Deliberately not inventing an identity format. The stack settling in 2026 —
OAuth 2.1 with PKCE, JWT bearer assertions for service-to-service, MCP for tool
invocation, signed identity tokens carrying the delegation chain — is what a
`key_binding` should eventually point at. GitLocus's contribution is recording
what the chain *did*, not issuing the credentials.

## Alternatives rejected

**Identity per instance.** Reputation becomes unusable; the graph grows without
bound in the least useful dimension.

**Ignore the model in the triple.** Tempting for simplicity, but a harness that
silently swaps to a weaker model would inherit standing it did not earn, and the
change would be invisible.

**Wait until swarms are common.** The cost of this change is proportional to the
history already recorded under the old shape. It only ever gets more expensive.
