<!-- SPDX-License-Identifier: Apache-2.0 -->
# Non-goals

Things this project deliberately will not do. Written down so they can be argued
with rather than rediscovered.

## We do not detect AI authorship

The most important entry, and the one most often expected of a project in this
space.

Final source text carries no reliable signal about how it was produced.
Attribution can be stripped, model output can be edited into anything, and human
code is now routinely model-assisted. A detector would be wrong often, wrong
invisibly, and wrong in a way that punishes honest contributors who disclose while
rewarding those who do not.

We track **who is answerable** and **what can be proven**. See
[ADR 0002](adr/0002-no-ai-authorship-detection.md).

## We are not building a git host

Git hosting is a solved, capital-intensive commodity, and GitHub is actively
shipping into the agent space (Agent HQ, Agentic Workflows). Building a forge
first would mean spending years on the part that is not the problem.

The evidence, policy and trust layer is forge-external. See
[ADR 0001](adr/0001-evidence-not-a-forge.md).

## We do not auto-merge on machine judgement

Deterministic checks can say a change is *not yet ready*. They cannot say it is
*right*. The gap between those is where product intent, architecture and judgement
live, and it does not close as models improve — it moves.

`satisfied` means the policy's conditions are met, including whatever human
attestation the policy demanded. It does not mean a machine decided the change was
good.

## We do not store a reputation score

Standing is derived from evidence history at evaluation time. A persisted number
becomes a target, and generating evidence that moves it is cheap — which is the
premise this whole project starts from.

## We do not replace CI, Sigstore, SLSA, or Vouch

Each of those solves a problem well. Reimplementing any of them would produce a
worse version and a compatibility burden.

- CI produces the deterministic evidence; we consume it.
- Sigstore signs; we will carry its bundles.
- SLSA Source Track defines the slot; we aim to be a format that fits it.
- Vouch already solved the social trust half; we read `VOUCHED.td`.

## We do not do AST-aware merging

Not now, possibly not ever. A structural merge that silently produces incorrect
behaviour is far worse than a text conflict a human resolves, because the text
conflict is *loud*. This is a correctness problem wearing a convenience problem's
clothing.

## We do not index repositories for semantic search

Vector search over code is not the bottleneck. Agents already have their own
retrieval, and adding a second, worse one attached to a policy engine would be
scope creep dressed as capability.

## We are not a chatbot on top of git

No conversational interface to repository state, no "ask your repo" surface. The
output of this system is a verdict and a queue, both machine-readable.

## We do not expose agent reasoning

Provenance means structured, verifiable facts about what ran and what passed. It
does not mean chain-of-thought, prompts, or model parameters. Those are neither
verifiable nor safe to publish, and treating them as evidence would confuse
narrative with proof.

## We do not sell seats per agent

Deferred rather than refused: the economics are a Stage 3 question. But the
architecture must not assume one identity equals one paying human, because a team
of two people may run two hundred short-lived agents. Nothing in the core model
counts actors for billing.
