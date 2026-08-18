<!-- SPDX-License-Identifier: Apache-2.0 -->
# Threat model

A system that claims to establish trust must say what it does not defend against.
This document is as important as the specification, and shorter on purpose.

## What GitLocus is trying to protect

**Maintainer attention.** Not code, not secrets, not availability — those are
protected by other things. The asset here is a finite human resource that is
currently spent in the wrong order.

The failure this project exists to prevent: a maintainer spends an hour on a
plausible change that could have been shown to be unmergeable in six seconds.

## Trust boundaries

| boundary | who is on the far side | what crosses it |
|---|---|---|
| Contributor → repository | Anyone, including agents nobody will answer for | A Contribution and self-asserted claims |
| CI → verdict | The forge's runner | Deterministic Evidence |
| Model reviewer → verdict | A probabilistic system | Assessed Evidence, binding on nothing |
| Human → verdict | A named person | Attested Evidence |
| Policy author → verdict | Whoever can merge to protected paths | The rules themselves |

The last row is the sharpest one: whoever can change the policy can change every
other answer. This is why `.gitlocus/**` and `.github/workflows/**` require
maintainer standing in this repository's own policy, and why the ruleset restricts
pushes to those paths to code owners.

## Threats this defends against

**T1 — Stale evidence replay.** A contributor gets green checks, then force-pushes
different code. *Defence:* evidence is bound to `subject_digest`; evidence for
another revision cannot satisfy a requirement. Covered by test
`clause_2_evidence_for_another_revision_cannot_satisfy_a_requirement`.

**T2 — Laundering a model's opinion into an approval.** A confident-sounding AI
review is treated as a passing check. *Defence:* class separation is enforced in
the type system, not the presentation layer. An implementation that allows this is
non-conformant by definition.

**T3 — Weak-rule shopping.** A contributor touches a leniently governed file
alongside a strictly governed one, hoping the lenient rule applies. *Defence:*
matching rules union their requirements and take the strictest approvals and tier.

**T4 — Inconclusive-as-pass.** A check crashes and the absence of a failure reads
as success. *Defence:* `inconclusive` is unmet.

**T5 — Unattributable agent work.** An agent opens changes nobody will answer for.
*Defence:* `ActorKind::Agent` carries no responsible human, and policies can
require a tier such an actor cannot reach.

**T6 — Non-determinism hiding a bug.** A verdict that varies between runs cannot be
audited. *Defence:* verdicts are pure functions; evidence ordering is tested.

## Threats this does NOT defend against

Stated plainly, because a security document that only lists wins is marketing.

**Lying deterministic evidence.** If CI is compromised, it can emit
`deterministic/pass` for anything. GitLocus verifies the *shape* of a claim, not
the honesty of its producer. Signing (v0 does not yet specify an envelope) narrows
this to "whoever holds the key", which is narrower but not closed.

**A malicious maintainer.** Whoever can approve and can edit the policy can do
anything. This is not solvable inside the tool; it is what SLSA Source Track L4
two-party review addresses, at the forge layer.

**Genuinely good code with bad intent.** Every check can pass on a change that is
subtly harmful. GitLocus decides *what deserves a human's attention*, never *what
is safe*. Nothing here is a substitute for review.

**Compromised dependencies of the checks themselves.** A malicious action in a
workflow can make everything green. This is why actions are SHA-pinned, why
Dependabot runs with a cooldown, and why `zizmor` audits the workflows — but those
are mitigations, not guarantees.

**Sybil trust.** Nothing prevents many `unknown`-tier identities. The design
response is that `unknown` should be able to *propose* freely and *bind* nothing,
so cheap identities buy no leverage. Whether that holds under pressure is
untested.

**Denial of service by volume.** Submitting enormous numbers of contributions
still costs the forge and the CI budget. Ranking makes the queue survivable; it
does not make the compute free.

**Prompt injection into an assessed reviewer.** A contribution can contain text
aimed at the model reviewing it. This is a real and unsolved attack — and it is
precisely why assessed evidence binds nothing. A successful injection produces a
misleading advisory note, not a merge.

## Assumptions

1. The forge correctly reports which paths a change touches.
2. Git content digests are collision-resistant for this purpose.
3. The policy at the evaluated revision is the policy that should apply. (An
   attacker who can change the policy in the same change it governs defeats this;
   path restrictions on `.gitlocus/**` are what close it.)
4. Deterministic checks are actually deterministic. Flaky tests silently violate
   this and are a correctness problem here, not just an annoyance.

## Open problems

Honest gaps, tracked rather than buried:

- **No signing envelope in v0.** Evidence is structurally validated but not
  cryptographically bound to its producer. Sigstore keyless is the intended
  answer; agents on laptops and agents in CI have very different key stories.
- **Blast radius is approximated by path globs**, which is crude. Something better
  would need to stay deterministic to be usable here.
- **Tier assignment is out of scope in v0.** How an actor comes to hold
  `contributor` is left to the implementation, which means it is currently the
  weakest link in the chain.
