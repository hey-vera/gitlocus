<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0003 — Evidence classes are enforced in the type system

- **Status:** accepted
- **Date:** 2026-08-18

## Context

A change arrives carrying: a passing test suite, a clean linter, and a model
review saying "no concerns". All three are claims about the change. Existing tools
render them as a list of checks, most with a green tick.

They are not the same kind of thing. Two of them can be re-run by anyone with the
inputs and will produce the same answer. The third is a sample from a distribution
and may differ on the next run, on a different model, or on the same model given
the same input twice.

Collapsing them into one confidence number — the obvious design — destroys exactly
the information a maintainer needs to decide where to spend the next hour.

There is a second, sharper reason. A contribution can contain text aimed at the
model reviewing it. Prompt injection against an assessed reviewer is a live,
unsolved attack. If assessed evidence could bind, a successful injection would
produce a merge.

## Decision

Evidence carries a mandatory `class`, and the class determines what it can do:

| class | definition | can satisfy a requirement |
|---|---|---|
| `deterministic` | Reproducible by any party with the same inputs | **yes** |
| `assessed` | Produced by a heuristic or a model | **never** |
| `attested` | A human accepted responsibility | approvals only |

The restriction lives in `Evidence::is_binding_for`, not in a rendering layer.
There is no threshold, no flag, and no configuration that lets an `assessed`
record satisfy a requirement. The specification declares an implementation that
permits it **non-conformant**.

Assessed evidence is still collected and still shown, in a dedicated `advisory`
field on the verdict. It is frequently the most useful thing on a pull request.
It just never decides anything.

## Consequences

**Good.** A prompt injection against the model reviewer produces a misleading
advisory note rather than a merge — the blast radius is bounded by construction.
The verdict stays auditable, because everything binding can be independently
re-run. And the model's output is not wasted: it is surfaced where a human will
read it.

**Bad.** Maintainers who want a model to auto-approve trivial changes cannot have
it. That is a real cost and we are choosing to pay it. The counter-argument — that
today's models are good enough — misses that the guarantee here is structural, and
a structural guarantee that holds only while the model behaves is not a guarantee.

**Load-bearing.** Almost every other decision in the project depends on this one.
It is why `advisory` exists on the verdict, why the CI is arranged in three tiers,
and why the threat model can bound prompt injection at all.

## Alternatives rejected

**Confidence thresholds.** "Assessed evidence binds above 0.95" reintroduces
everything this decision removes, and puts the security boundary on a number
nobody can calibrate.

**Trusted model allowlist.** Makes the guarantee depend on vendor behaviour, and
provides no defence against injection, which is a property of the input rather
than the model.

**Class as presentation metadata only.** This is what most tools do today, and it
is why a green tick from a model reviewer sits next to a green tick from a test
suite looking identical.
