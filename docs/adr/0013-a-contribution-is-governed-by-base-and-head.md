<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0013 — A contribution is governed by base and head together

- **Status:** accepted
- **Date:** 2026-08-19

## Context

[ADR 0006](0006-the-gate-must-resist-what-it-gates.md) says the gate must resist
what it gates, and gives the principle: **any check is only worth what the thing
defining it is worth.** The policy is the thing defining every check.

The specification said a verifier MUST evaluate the policy at the revision under
evaluation, and the gate did exactly that — it read `.gitlocus/policy.yml` out of
the pull request's own working tree. So a contribution that deleted the rules
governing it was judged by a document with those rules removed.

This was not theoretical. Run against the repository as it stood, an
`unknown`-tier outsider whose only change is replacing the policy with
`rules: []`:

```
rules matched: none
verdict      : satisfied: every requirement met
rank         : confidence 1.00, 0 approval(s) outstanding
exit: 0
```

No build, no tests, no lint, no approval, no standing. The same contribution
against the policy at the base revision:

```
rules matched: baseline, ci-and-policy
verdict      : blocked: contributor does not clear Maintainer
  unmet      : build, fmt, lint, tests, workflow-audit  (all Missing)
exit: 1
```

Identical inputs, opposite answers, and the wrong one was the one shipping.

Two things made this hard to see. The evidence was honest — there was nothing
anomalous to notice, only a rule that was absent. And conformance clause 6 named
the property but **had no test**, because it reads as a statement about which
file to open rather than a claim about a verdict.

The protections that looked like they covered this did not. The `ci-and-policy`
rule holds `.gitlocus/**` to maintainer standing, but only in whichever copy of
the policy is being read. CODEOWNERS routes review, and review was bypassed.

## Decision

**The governing policy is the policy at `base_digest` together with the policy at
the revision under evaluation.** Both, always, where both exist.

No new evaluation semantics were needed. Evaluation already unions required
checks, takes the strictest approvals and tier, and requires every signer glob
constraining a check to match — so concatenating the rules of two documents is
exactly the intended reading, and the result is provably never weaker than either
input. `CompiledPolicy::merged` is a concatenation and nothing else.

Rules from the base revision are labelled `governing:`, so a verdict says
`blocked by governing:ci-and-policy` rather than naming a rule the reader cannot
find in the file in front of them.

**Absence and failure are different.** A contribution that first introduces a
policy has none at the base revision, and is correctly governed by what it ships
alone. A policy that exists and cannot be read or parsed blocks. A verifier that
silently treated the second as the first would reopen the hole in the one case
where it matters most.

## Consequences

**Good.** The asymmetry falls out in the useful direction: a rule a contribution
adds binds that contribution immediately, so a policy cannot be tightened and
dodged in one change; a rule it removes keeps binding until the change removing
it has itself been accepted. This is also what makes
[ADR 0006](0006-the-gate-must-resist-what-it-gates.md)'s privileged-path
mechanism worth anything at all — before this, a `checks` rule could be deleted
by the contribution it was meant to constrain, in the same breath.

**Bad — a loosening takes one extra round trip.** A maintainer relaxing a rule is
still held to the old one for the pull request that relaxes it. That is the cost
of the property and it is not avoidable: distinguishing a legitimate loosening
from an attack is exactly the judgement the policy exists to avoid making.

**Bad — the common case now compiles two documents.** Most contributions do not
touch the policy, so base and head are usually identical. Merging a policy with
itself decides the same way — required checks are a set, so nothing
double-counts — but it is measurable work for no benefit in the common case, and
it is accepted rather than optimised because a fast path here would be a second
code path through the only security-relevant decision in the system.

**Verdicts stay pure.** The inputs become (base policy, head policy,
contribution, evidence), which is still a pure function and still
content-addressable. Ordering of the evidence array and of the policies does not
change the verdict, and there is a test.

## Alternatives rejected

**Evaluate the base policy only.** Simpler, and it is what forge branch
protection effectively does. Rejected because a rule a contribution adds would
then never apply to the contribution adding it, which is a second hole in the
opposite direction.

**Hold `crates/**` to maintainer standing instead.** The cheaper half of
[#21](https://github.com/hey-vera/gitlocus/issues/21). It protects the evaluator
and does nothing about the policy document, which is the more direct lever.
Worth doing as well, not instead.

**Rely on the forge.** CODEOWNERS and required review can protect the policy path
without any of this. Rejected because it is forge-specific, unavailable to a
single-maintainer repository, and — decisively — it makes the *merge* safe while
leaving the *verdict* wrong. The verdict is the product.
