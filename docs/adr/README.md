<!-- SPDX-License-Identifier: Apache-2.0 -->
# Architecture Decision Records

Why things are the way they are. Each record states the context, the decision, and
what it costs — including the alternatives that were rejected and why.

A decision recorded here is not permanent. It is *explicit*, which is what makes it
possible to argue with later. If you think one is wrong, open a discussion and say
which part of the context has changed.

| # | decision | status |
|---|---|---|
| [0001](0001-evidence-not-a-forge.md) | Build the evidence layer, not a forge | accepted |
| [0002](0002-no-ai-authorship-detection.md) | No AI authorship detection | accepted |
| [0003](0003-evidence-classes.md) | Evidence classes are enforced in the type system | accepted |
| [0004](0004-rust-core-shared-by-cli-and-server.md) | One Rust core, shared by the CLI and the server | accepted |
| [0005](0005-evidence-classes-survive-better-models.md) | Evidence classes survive better models | accepted |
| [0006](0006-the-gate-must-resist-what-it-gates.md) | The gate must resist what it gates | accepted |
| [0007](0007-actor-identity-is-a-delegation-chain.md) | Actor identity is a delegation chain, not a name | accepted |
| [0008](0008-authorship-is-declared-not-detected.md) | Authorship is declared, not detected | accepted |
| [0009](0009-trust-is-earned-from-merged-history.md) | Trust is earned from merged history, not asserted | accepted |
| [0010](0010-an-attestation-needs-someone-to-attest.md) | An attestation needs someone to attest | accepted |

## Reading order

New here? [0001](0001-evidence-not-a-forge.md) says what this project is,
[0003](0003-evidence-classes.md) says what makes it work, and
[0005](0005-evidence-classes-survive-better-models.md) says why that will still
be true in five years.

[`../AGENT-ERA.md`](../AGENT-ERA.md) collects the problems these decisions are
answers to, including the ones still unsolved.

## The load-bearing ones

Two records are not ordinary decisions. Changing them changes what this project
is:

- **[0003](0003-evidence-classes.md) + [0005](0005-evidence-classes-survive-better-models.md)** —
  assessed evidence can never bind. This is the source of the only real security
  property here: a successful prompt injection against a model reviewer produces
  a misleading note, not a merge. It will be argued against every year as models
  improve. 0005 exists to answer that argument once.
- **[0002](0002-no-ai-authorship-detection.md) + [0008](0008-authorship-is-declared-not-detected.md)** —
  we never infer authorship from source text; we record what a named party
  declares. Read together, or 0008 looks like a reversal of 0002 when it is the
  opposite.

## Writing a new one

Copy the shape of an existing record. Number sequentially. Keep it short — if it
runs past a page, the decision is probably two decisions.

A record must include what the decision **costs**. A record with only benefits is
not a decision, it is an advertisement.

Write down the argument you expect to face later, and answer it. That is what
makes a record worth having in three years, when the person who made the decision
is not in the room.
