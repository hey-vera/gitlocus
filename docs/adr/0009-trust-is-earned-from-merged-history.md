<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0009 — Trust is earned from merged history, not asserted

- **Status:** accepted
- **Date:** 2026-08-18

## Context

The tier ladder had a missing rung. `unknown` is the default, `vouched` comes
from a `VOUCHED.td`, and `maintainer` comes from forge permission. Nothing
produced `contributor` — the tier for someone who has landed work here but holds
no repository permission, which in open source is most of the people who matter.

Leaving it unreachable meant the ladder had a gap exactly where the interesting
population sits, and it left [ADR 0007](0007-actor-identity-is-a-delegation-chain.md)
without a promotion mechanism: standing attaches to a durable triple, but nothing
said how a triple ever rises above the floor.

The obvious implementation is a trap, and it is worth writing down so nobody
re-invents it.

> **Do not derive trust from the git author email.**

`git log --author=` is one command away and it is unsound. The author field is
free text that anyone can set to anything; `git commit --author="Linus
Torvalds <torvalds@linux-foundation.org>"` requires no permission at all. Trust
derived from it is trust anyone can mint. This project's design consistently
holds that a claim in a document is not evidence, and the author field is
precisely a claim in a document.

## Decision

**Standing is derived from contributions that were actually merged, attributed
by an authenticated identity, never by a self-asserted one.**

Concretely, in the GitHub integration: the count of merged pull requests
authored by the authenticated account, which the forge attributes rather than
the commit does. A project sets the threshold:

```yaml
require:
  min_tier: contributor        # what the rule demands
```

with the promotion threshold configured where the tier is derived, not in the
policy — the policy states what standing is required, not how anyone acquires it.

**Nothing is stored.** The count is recomputed at evaluation time from history
the forge already holds. This preserves the rule from
[the specification's §5](../../spec/README.md): reputation is *derived*, never a
persisted score. A stored number is a target to optimise; a count of merged work
is only movable by doing the work.

**Derivation is layered, strongest signal first:**

| signal | tier | why it is sound |
|---|---|---|
| repository permission | `maintainer` | the forge asserts it, and it is unforgeable by the contributor |
| merged history, authenticated | `contributor` | landing work required passing the gate |
| `VOUCHED.td` | `vouched` | someone already trusted put their name to it |
| nothing | `unknown` | the safe default |

A denouncement caps the result regardless of anything above it.

## Consequences

**The ladder is complete and every rung is earned.** No tier can be reached by
asserting anything about yourself.

**It is forge-dependent, and that is a real cost.** Merged history is attributed
by whatever hosts the repository, so the derivation lives in the forge
integration rather than in `locus-core`. That is consistent with
[ADR 0004](0004-rust-core-shared-by-cli-and-server.md) — the core stays pure and
takes a tier as input — but it means each forge needs its own derivation, and a
mailed patch series has none.

**The durable answer is the evidence ledger, not the forge.** Once GitLocus holds
a signed history of contributions it has evaluated, standing can be derived from
*that* — portable across forges, and grounded in evidence this system verified
rather than in a third party's bookkeeping. Stage 1 work. The forge derivation is
the stopgap that makes the tier usable now, and it should be replaced rather than
extended.

**Cross-repository trust remains deliberately out of scope.** Whether standing
earned in one project should mean anything in another, and who would be entitled
to say so, is a governance question rather than a technical one. It stays in the
specification's open questions.

## Alternatives rejected

**Git author email.** Spoofable by anyone, for free. Covered above.

**Signed commits as the signal.** Sounder than the author field, but it measures
whether someone configured signing, not whether they contributed. Most valuable
contributors to most projects do not sign commits.

**A stored reputation score.** Explicitly refused by the specification. The
moment it exists, it is a number to farm, and generating evidence that moves it
is exactly the cost this project assumes has collapsed.

**Time-based promotion.** "Account older than N days" measures patience, not
contribution, and costs an attacker nothing but a calendar.
