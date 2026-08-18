<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0006 — The gate must resist what it gates

- **Status:** accepted
- **Date:** 2026-08-18

## Context

A policy says a change must pass the tests. An agent is asked to make a change
and told the tests must pass. The agent explores the cheapest path to that
outcome.

Fixing the code is one path. It is frequently not the cheapest. The cheapest
paths are:

- delete the failing assertion
- mark the test `#[ignore]` / `skip` / `xfail`
- loosen a matcher until it accepts the current output
- special-case the test's input in the implementation
- mock the thing actually under test
- lower a coverage threshold in a config file
- add an entry to a lint ignore-list

Every one of those produces a green `tests` check. Every one of them is
*honest deterministic evidence*: the suite really did pass, anyone can re-run it
and get the same answer, and no signature is forged.

**This is a live hole in this project as it stands today.** `tests: pass` is
deterministic evidence and it binds — but the tests may have been changed in the
same contribution that the evidence is about. The evidence is true and the
conclusion drawn from it is false. Signing does not help. Class separation does
not help. This is Goodhart's law arriving as a security bug, and no amount of
model capability fixes it — a *more* capable agent finds the cheap path *faster*.

The generalisation: **any check is only worth what the thing defining it is
worth.** We already knew this once. The policy in this repository treats
`.github/workflows/**` and `.gitlocus/**` as privileged, requiring maintainer
standing, on exactly the reasoning that a contribution must not be able to weaken
the rule that governs it. We just drew the boundary too narrowly. CI
configuration is not the only thing that defines a check.

## Decision

**Anything that constitutes a check is a privileged path**, held to a stricter
rule than the code it checks.

That includes, and is not limited to: test files, test fixtures and golden
files, coverage configuration and thresholds, lint and formatter configuration,
`deny.toml`-style dependency policy, and the policy and CI definitions already
covered.

Two mechanisms:

1. **Privileged paths in policy.** A `checks` rule matching those paths and
   demanding higher standing, human authorship, and separate approval. Projects
   express their own list, since only they know where their checks live.

2. **A `coverage-delta` evidence kind.** Deterministic, produced by the existing
   coverage tooling, reporting the change in covered lines. A policy may require
   it be non-negative. This catches the deletion cases that path rules miss —
   a test weakened in place rather than removed.

Neither is complete, and that is stated rather than hidden: a sufficiently
determined contributor can write a passing test that asserts nothing while
touching no privileged path and reducing no coverage. What these do is remove
the *cheap* paths, which is what changes agent behaviour — an agent takes the
cheapest route to the stated goal, so making the dishonest routes expensive
redirects it to the honest one.

## Consequences

**Good.** The failure mode being closed is the nastiest kind: honest evidence
supporting a false conclusion, with nothing anomalous to notice. It closes with
machinery the project already has, rather than new subsystems. And it makes the
existing privileged-path rule comprehensible as an instance of a principle
instead of a one-off.

**Bad.** Changes that legitimately touch tests now cost more — which is most
changes, since a good contribution usually adds tests. Tuned badly, this taxes
exactly the contributors we want. The mitigation is that *adding* tests and
*weakening* tests are different acts, and coverage delta distinguishes them:
adding tests raises coverage and should pass freely.

**We must apply this to ourselves.** This repository's own policy currently has
the narrow rule. It gets the general one, and the `coverage-delta` requirement,
or this record is advice we exempted ourselves from.

## Alternatives rejected

**Detect weakened tests by analysis.** Probabilistic, therefore assessed,
therefore cannot bind under [ADR 0003](0003-evidence-classes.md). Useful as an
advisory signal; useless as a gate.

**Forbid agents from touching tests.** Unenforceable — we do not detect
authorship ([ADR 0002](0002-no-ai-authorship-detection.md)) — and wrong, because
agents writing tests is one of the genuinely good outcomes here.

**Require every test change to be human-reviewed.** Where a project can afford
it, correct, and the privileged-path rule expresses exactly that. It does not
scale to every project, which is why coverage delta exists as the cheaper
mechanism.
