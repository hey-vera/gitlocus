<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0015 — Mutation testing is what resists gate gaming

- **Status:** accepted
- **Date:** 2026-08-19
- **Amends:** [0006](0006-the-gate-must-resist-what-it-gates.md)

## Context

[ADR 0006](0006-the-gate-must-resist-what-it-gates.md) is right about the
problem and wrong about both of its mechanisms. The problem it names is real and
unchanged: an agent told the tests must pass takes the cheapest route to that
outcome, weakening a test is usually cheaper than fixing the code, and the
resulting evidence is *honest* — the suite really did pass.

Its two answers do not hold.

**Privileged paths cannot express "the tests" here.** 0006 proposes a `checks`
rule matching test files, and closes by insisting the rule be applied to this
repository or the record is advice we exempted ourselves from. That is
structurally impossible: this project's tests are inline `#[cfg(test)] mod tests`
inside `crates/gitlocus-core/src/*.rs`, so no path glob selects the tests without
also selecting the implementation they test. A rule over `crates/**` is not a
privileged-path rule, it is a closed repository — the same finding that decided
[ADR 0014](0014-the-gate-is-built-from-the-base-revision.md).

This is not peculiar to us. Inline unit tests are idiomatic Rust, so the
mechanism silently does nothing for a large fraction of adopters while appearing
to be in force, which is the worse failure shape.

**Coverage delta does not do what 0006 claims.** It says of `coverage-delta`:
*"This catches weakening in place, which path rules miss."* It does not. Line
coverage moves when a test is **deleted**. It does not move when an assertion is
**loosened**:

```rust
assert_eq!(result, Verdict::Blocked);   ->   assert!(matches!(result, _));
```

Same lines executed, same coverage, no signal. Coverage delta catches removal,
and removal is what path rules already catch. The one case 0006 claimed it added
is the one case it misses — a claim stronger than the mechanism, which is the
failure this project tabulates against itself in `AGENTS.md`.

## Decision

**Require a deterministic mutation check over the changed lines.**

A mutant is a small edit to the implementation — flipping a comparison, replacing
a return value. If the tests still pass with the mutant in place, the mutant
*survived*, and the tests do not actually constrain that behaviour. A loosened
assertion produces surviving mutants immediately, which is exactly the
measurement 0006 wanted and did not get.

[`cargo-mutants`](https://github.com/sourcefrog/cargo-mutants) supports
`--in-diff`, built for pull-request gating, so cost scales with the size of the
change rather than the size of the codebase.

**A timeout is `inconclusive`, never a pass.** Mutation testing runs the suite
many times and a mutant that loops forever is indistinguishable from one that is
merely slow. Timing-dependent results cannot be deterministic evidence, so the
job fails on either surviving mutants or timeouts, and the log distinguishes
them. Failing is stricter than inconclusive, so nothing is laundered into a pass.

**Privileged paths stay, demoted.** They work where a project keeps its tests in
their own tree, and they remain the right answer for CI configuration and the
policy itself — paths that define checks without being checks. What changes is
that they are no longer claimed to be the general mechanism.

**The tree does not have to be clean for the check to be fair.** Because
`--in-diff` mutates only the lines a contribution touches, a contributor who
touches an untested line is asked to test it, and nobody is charged for a gap
they did not create. That is what makes this adoptable on an existing codebase
rather than only on a new one, and it is why no global mutation score is
required or recorded — a score would be a number to farm, which
[the specification already refuses](../../spec/README.md) for standing.

The check is scoped to `gitlocus-core` for now. That is not the same kind of
exclusion: the CLI has *no* behavioural test harness at all, so a contributor
touching it would have to build one from scratch rather than add a test beside an
existing one. The scope is written down here and in the workflow rather than
quietly configured, and removing it is [#34](https://github.com/hey-vera/gitlocus/issues/34).

## What it found immediately

Run against this repository before any of it was wired into CI, mutation testing
reported that **`crates/gitlocus-cli` has no behavioural tests at all** — 24
surviving mutants, every function in `main.rs` and `git.rs` replaceable by a
constant with the suite still green. Two of them are not cosmetic:

```
crates/gitlocus-cli/src/git.rs:61:5:  replace changed_paths -> Result<Vec<String>> with Ok(vec![])
crates/gitlocus-cli/src/main.rs:438:5: replace verify -> Result<ExitCode> with Ok(Default::default())
```

A contribution with no changed paths matches no rule, so the first turns every
verdict into `satisfied`. The second makes the exit code unconditionally zero,
and the exit code is what CI branches on. Either one silently disables the gate,
and the entire existing suite passes with them applied.

A contribution with no changed paths matches no rule, so the first turns every
verdict into `satisfied`. The second makes the exit code unconditionally zero,
and the exit code is what CI branches on. Either one silently disables the gate,
and the entire existing suite passes with them applied. Tracked as
[#34](https://github.com/hey-vera/gitlocus/issues/34) rather than fixed here.

In `gitlocus-core` it found three more, all fixed rather than excluded:
`Contribution::id` could return an empty string or `"xyzzy"` — the identifier
printed in every verdict — and `PolicyError`'s `Display` could render nothing,
which is the only diagnostic a contributor with a broken policy ever sees. Three
tests later, all three mutants are caught.

None of this was visible to any check this repository already ran. It is the
argument for this record, made by the mechanism the record proposes, in minutes.

## Consequences

**Good.** It measures the property actually wanted — do the tests constrain the
implementation — rather than a proxy for it. It works regardless of where a
project puts its tests, which is what privileged paths could not do. And adding
tests is rewarded rather than taxed, so it does not penalise the contributors
0006 worried about.

**Bad: it is slow, and it is slow in proportion to the test suite.** This suite
runs in well under a second, so diff-scoped mutation is affordable here. A
project with a ten-minute suite cannot run this per pull request, and this record
does not pretend otherwise. Those projects get the advisory signal or a nightly
run, not a gate.

**Bad: a new tool in the trusted path.** The check is only as trustworthy as
`cargo-mutants`, which is a dependency this repository does not otherwise have.
It is pinned and it runs in CI, where a compromised version could report success
falsely — the same exposure every other check already carries, and no worse.

**Still incomplete, and 0006 was right to say so.** A determined contributor can
write a test that executes code and pins current behaviour without asserting
anything meaningful, satisfying both coverage and mutation. What this removes is
the *cheap* path. An agent takes the cheapest route to the stated goal, so making
the dishonest routes expensive redirects it to the honest one — 0006's own
argument, now attached to a mechanism that supports it.

**We apply it to ourselves**, which is the part of 0006 that could not be
honoured before.

## Alternatives rejected

**Keep coverage delta as well.** It catches only what path rules already catch,
at the cost of another tool and another threshold to argue about. Mutation
subsumes it for the cases that matter.

**Detect weakened tests by analysis.** Probabilistic, therefore `assessed`,
therefore unable to bind under [ADR 0003](0003-evidence-classes.md). Unchanged
from 0006, and still correct.

**Require human review of every test change.** Correct where a project can
afford it, and expressible today as a privileged path. It does not scale, and it
is unavailable to a single-maintainer repository — which is this one.
