<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0018 — A claim about every input is tested over every input

- **Status:** accepted
- **Date:** 2026-08-20
- **Reinforces:** [0015](0015-mutation-testing-is-what-resists-gate-gaming.md)

## Context

Four of the seven invariants in AGENTS.md are universally quantified. Invariant 4
says a verdict has "no dependence on the order of the evidence array". Invariant 1
says assessed evidence must never satisfy a requirement, "not with a high score,
not with a confidence threshold, not behind a flag". `spec/README.md` §6 clause 5
requires "byte-identical verdicts for identical inputs", and §3.5 spells out that
no "ordering of the evidence array" may affect one.

All four were tested on inputs somebody chose.

That is not a criticism of those tests. A hand-written case is how a reader is
shown what a rule means, and `clause_5_evidence_order_does_not_change_the_verdict`
did that clearly: three records, reversed, compared as serialised bytes — the
right assertion. It is a statement about what an example can establish. An example
establishes that the rule holds for the example.

It did not hold. That test used **one** assessed record, so the `advisory` list it
produced had a single element and reversing the input could not change its order.
Two assessed records in a different order produced different verdict bytes for the
same set of inputs. `decision` was unaffected — nothing about the outcome was
wrong — but the verdict was not byte-identical, which is the property the whole
content-addressability argument rests on. Two agents holding the same evidence in
different orders would compute different cache keys for the same question.

The bug was three years of reasoning away from anything a reviewer would look for,
and a generated test found it in ninety-one cases.

## Decision

**Where a claim is quantified over all inputs, the test quantifies over all
inputs.** `proptest` is a dev-dependency of `gitlocus-core`, and
`crates/gitlocus-core/tests/properties.rs` holds one property per quantified
claim, each naming the invariant it is about.

Hand-written examples stay. They are the documentation of what a rule means and
they shrink better than any generated case for a reader; the properties are what
establish that the rule is true. Where a property finds something, the example
that missed it is strengthened so that it would have caught it too — as clause 5
was, from one assessed record to two.

**Totality is a property, and it is where the parsers get their coverage.**
`vouch.rs` documents itself as total: "a line this module cannot interpret is
ignored rather than rejected." `VOUCHED.td` and `.gitlocus/policy.yml` are both
read out of a contribution's own tree, so both are attacker-chosen. A property
over arbitrary strings asserting that neither parser panics is the same contract a
fuzzer would check, in the test suite that already runs on every pull request.

## Consequences

**Seven new packages** — `proptest`, `rand`, `rand_chacha`, `rand_core`,
`rand_xorshift`, `ppv-lite86`, `unarray`. All are MIT or Apache-2.0, so
`deny.toml`'s exact-match allow-list needed no change, which is the outcome that
made this cheap enough to do now. `--no-default-features --features std` keeps
`rusty-fork`, `tempfile` and `regex-syntax` out; the cost is that a panicking
property fails the test process rather than being isolated in a forked child,
which for a suite that is expected to pass is the right trade.

**A generated failure is harder to read than a chosen one.** Proptest's shrinking
helps and does not eliminate it. The mitigation is that every property carries a
comment naming the invariant, so a failure points at the sentence it falsifies
rather than only at an input.

**Properties are not deterministic across runs by default.** Proptest seeds from
entropy and writes a regression file when it finds a failure. Those files are
checked in when they appear: a `.proptest-regressions` entry is a specific input
that once broke an invariant, which is exactly the kind of test case worth
keeping. Until one exists there is nothing to check in.

**A property can pass by generating nothing interesting.** A generator whose
digests never collide would make the binding properties vacuous. This is the
failure mode with no automatic guard: `mutants` cannot see it, because a vacuous
property still kills mutants in the code it calls. The generators deliberately
use a four-value digest alphabet so that "evidence binds to the head" is a case
that actually occurs, and that reasoning is written where the generator is.

**Cost grows with the suite.** 256 cases per property by default, 512 for the
parsers. That is under a second today and will not stay that way; the number is a
knob and lowering it to keep CI fast would quietly weaken the check, which is the
move [ADR 0006](0006-the-gate-must-resist-what-it-gates.md) exists to stop. If it
becomes slow, the answer is a scheduled deeper run, not a smaller number on the
pull-request path.

## Alternatives rejected

**`cargo-fuzz` for the parsers, now.** It is the stronger tool: coverage-guided,
and it finds inputs no uniform generator reaches. Rejected for this change and
kept in #60, because it needs decisions this does not: where a corpus lives — a
corpus checked into the repository is a supply-chain surface of its own — and on
what cadence it runs. It cannot run per pull request, because a fuzz run is
timing-dependent and ADR 0015 is explicit that a timing-dependent result is
`inconclusive`, never a pass. A property over arbitrary strings gets the totality
contract under test today, in a check that already blocks merges, and does not
foreclose the stronger tool.

**Only strengthening the example tests.** Adding a second assessed record to
clause 5 fixes this bug and nothing else. The reason the bug existed is that
nobody thought of two assessed records, and thinking harder is not a mechanism —
it is the thing LEARNINGS.md is a list of the failures of.

**Putting the properties in the existing conformance suite.**
`tests/conformance.rs` is the numbered mapping from `spec/README.md` §6 to
executable clauses, and an implementer reads it as that mapping. Mixing generated
tests into it would make a clean document harder to read. Properties reference the
clause they support and live beside it.

**Generating the policy as well as the evidence.** Tempting, and it would explore
far more of `evaluate`. Rejected for now because the properties assert against
*this repository's own policy*, read from `.gitlocus/policy.yml` — the one whose
behaviour anyone can check against a file in the tree. A generated policy would
make a failure a claim about a document that does not exist. Worth revisiting once
there is a second real policy to test against.
