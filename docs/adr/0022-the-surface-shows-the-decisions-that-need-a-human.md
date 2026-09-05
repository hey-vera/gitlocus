<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0022 — The surface shows the decisions that need a human

- **Status:** accepted
- **Date:** 2026-09-05

## Context

The console at locus.heyvera.org is a scenario tool: paste a policy, a
contribution and some evidence, and watch the verdict change. It is the right
surface for understanding the model and the wrong one for a product — #53 says
so — and with [0019](0019-the-locus-is-identity-standing-and-the-decision.md) the
product has a surface to design: what a human sees when they log in.

GitHub shows everything. Twenty years of accretion produced a file browser,
issues, discussions, projects, code review, releases, packages, stars,
notifications, and a pull request page that renders all of it at once. That
surface is enormous because it grew, not because a human needs it in order to
decide. Stage 2's done condition already takes the other position — a maintainer
with a hundred open contributions can tell in under a minute which five to look
at — and [THREAT-MODEL.md](../../THREAT-MODEL.md) names the asset: maintainer
attention.

The seed this record was asked to argue with rather than accept: *GitHub shows you
everything; GitLocus should show you only the decisions that need a human.* Argued
with, it holds, with one addition. A human also needs to see the trust they have
extended — every agent acting in their name, and what it may do — because that
screen is what decides whether the platform can be trusted at all.

[0004](0004-rust-core-shared-by-cli-and-server.md) said the web interface would be
TypeScript and React, generated from the OpenAPI contract, and that the human
interface would use the same API as agents. #53 later constrained the console to
no build step and no framework. The maintainer chose the long-term stack for this
surface, so 0004 stands and #53's constraint is superseded; the cost is real and is
paid deliberately below.

## Decision

**Four screens, the same API agents use, and nothing else until a user cannot do
their job.**

| screen | who | shows |
|---|---|---|
| **Queue** | a maintainer | contributions across their repositories, ordered by the pure ranking (#51); each row carries the decision, what is outstanding, who is answerable at the chain's root, and `human_cost`. Blocked work is collapsed by default: it is the agent's job, not the human's. |
| **Grants** | an operator | every delegation they have issued — agent triple, repositories, ceiling, acts, expiry, last use — with revocation one action away, and the consent screen through which a new one is issued ([0020](0020-identity-is-federated-standing-and-delegation-are-native.md)). |
| **Contribution** | anyone with access | one verdict, with the chain that produced the actor, the evidence with its classes visibly distinct, advisory findings marked as binding on nothing, and the scenario tool underneath it as "recompute this offline". |
| **Repository** | a maintainer | what the policy at the base revision demands, the recent ledger, and the licence-integrity view when #52 exists. |

No file browser, no code view, no issues, no discussions, no stars, no review
interface. Every one of those is a link to the backend, which has them. If a user
cannot do their job here without one of them, that is the signal to revisit this
record, and the revisit names the screen it would have to be.

**Stack.** TypeScript and React, built with Vite, with a client generated from the
evaluator's OpenAPI contract and the ledger's. `web/` stays AGPL-3.0-only. The
terms under which a build step is acceptable in this repository:

- the lockfile is committed and installs use it exactly (`npm ci`), with the Node
  version pinned;
- the bundle is built in an unprivileged CI job, and the release workflow attests
  the artifact it is handed — it never runs the build itself, because that
  workflow is the highest-privilege thing here and a package registry inside it is
  the exposure [LEARNINGS.md](../../LEARNINGS.md) warns about;
- `just licence-headers` covers `web/**/*.ts` and `web/**/*.tsx`, so a source file
  under `web/` without the AGPL identifier fails CI;
- the content-security policy in the Caddy site block is unchanged: the console
  loads nothing from a third party.

**Same API.** The console calls `/v0/` and `/v1/` exactly as an MCP client would.
There are no private endpoints for the web, so nothing a human can do is
unavailable to an agent under a grant that permits it, and nothing an agent can do
is hidden from the human who granted it.

## Consequences

**People will ask where the code is.** The answer is a link to the backend, and
some will leave. That is the cost of a surface small enough to read in a minute,
and it is accepted.

**A build step, the first in this repository.** A second toolchain to pin and
audit, a lockfile to keep current, and a dependency tree that dependency review
must cover. The terms above are what make it acceptable; loosening any of them is
the move [0006](0006-the-gate-must-resist-what-it-gates.md) exists to stop.

**The queue is only as good as #51.** Its third ordering key — whether a
nearly-ready contribution outranks a trivially-ready one — is undecided. Until it
is, the queue is a list with a decision column, and the Stage 2 done condition is
not met.

**The scenario tool survives.** #53 was right that losing it to a dashboard would
trade the thing that explains the product for the thing that uses it. It lives on
the Contribution screen as the recompute path.

## Alternatives rejected

**Vanilla HTML, no build step.** The console today, and #53's constraint. It
removes the supply-chain cost entirely, and it was the recommendation put to the
maintainer. Rejected by the maintainer for the long term: four screens with
authentication, revocation and a generated client are past what one file carries
well, and 0004 already made this call. The cost is stated above rather than argued
away.

**A GitHub-shaped surface.** Issues, code browsing, review. The commodity half,
again, and the prettier GitHub [0001](0001-evidence-not-a-forge.md) warned against.

**A dashboard that replaces the scenario tool.** Covered above.

**Private endpoints for the console.** Faster for one screen. Rejected because the
human interface using the same API as agents is 0004's condition for the claim
surviving contact with a deadline, and because a capability the console has and a
grant cannot express is a gap in the delegation model, not a convenience.
