<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0014 — The gate is built from the base revision

- **Status:** accepted
- **Date:** 2026-08-19

## Context

[ADR 0013](0013-a-contribution-is-governed-by-base-and-head.md) stopped a
contribution being judged by a policy document it wrote. The same problem exists
one level down and was left open: the gate **binary** was compiled from the pull
request it was judging.

A contribution touching `crates/gitlocus-core/src/policy.rs` changes the
evaluator that then decides whether that contribution may merge. That is a more
direct lever than the policy document, because it needs no rule to be deleted —
the rules can stay exactly as written while the code reading them changes
meaning. Nothing in `.gitlocus/policy.yml` covered `crates/**`.

The obvious cheap fix, and the one [#21](https://github.com/hey-vera/gitlocus/issues/21)
offered first, is a privileged rule holding `crates/**` to maintainer standing.
That is not a privileged-path rule for this repository, it is a closed
repository: `crates/**` is where all the source lives, so every outside
contribution would be blocked on standing it cannot have.

## Decision

**Build the gate from `base_digest`, in a detached worktree, and evaluate with
that binary.**

The contribution is still described from, and evaluated against, the head
revision. Only the thing doing the evaluating comes from the base. Combined with
0013, both halves of the decision — the rules and the code that reads them —
now come from a revision the contribution cannot alter.

Artifacts go to the workspace target directory rather than the worktree's own,
so the existing cache applies. Without that, every pull request pays for a cold
release build.

## Consequences

**Good.** The evaluator is no longer supplied by its subject. It also composes
with 0013 to give a single property worth stating plainly: *a contribution
cannot influence how it is judged*.

**Bad, and permanent: a change to this CLI needs two pull requests.** The base
binary does not know flags a pull request invents, so adding a flag and using it
in the workflow cannot happen in one change — the first adds the flag, the
second uses it once the first is in the base. This is exactly why 0013 had to
land before this record: it added `--governing-policy` and used it in one
change, which was only possible while the gate was still built from head.

That cost is worth naming precisely because it will be tempting to work around.
Anyone reaching for a conditional in the workflow to sniff whether the base
binary supports a flag is reintroducing the hole, since that conditional is
itself editable by the pull request.

**Bad: a bug fix in `locus` does not benefit the pull request that fixes it.**
The gate keeps using the old binary until the fix is in the base. For a gate
that is the correct direction, and for a contributor watching a known-broken
verdict block their fix it is still annoying.

**Neutral: the base tree must be buildable.** It always is, because it is a
merged revision that passed these same checks. A repository adopting GitLocus
mid-history could find otherwise, and the failure is loud.

## Alternatives rejected

**Hold `crates/**` to maintainer standing.** Cheapest, and it closes the same
hole for this repository at the cost of refusing all outside contribution. It
also does not generalise — see
[#28](https://github.com/hey-vera/gitlocus/issues/28), where the same
privileged-path mechanism turns out not to be expressible for this project's
tests either.

**Use the released binary instead of building.** Removes the build entirely and
is the right answer for adopters, which is what the composite action in
[#12](https://github.com/hey-vera/gitlocus/issues/12) will do. Rejected here
because `main` drifts ahead of the last release, so the gate would judge
contributions with an evaluator older than the policy they are evaluated
against — a subtler version of the same mismatch.

**Verify the gate binary's provenance instead of controlling its source.**
Attestation proves which workflow built a binary, not which revision it was
built from, so it does not answer this question at all.
