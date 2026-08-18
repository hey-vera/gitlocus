<!-- SPDX-License-Identifier: Apache-2.0 -->
# Roadmap

Five stages. Each one ships something usable on its own — if a stage would only
make sense once a later stage exists, it is in the wrong order.

The last stage is conditional and may never be built. That is deliberate.

## Stage 0 — Foundation ← *we are here*

The model, the specification, and a gate that runs on this repository's own pull
requests.

- [x] Core model: Actor, Contribution, Evidence, Policy, Verdict
- [x] Deterministic policy evaluation, shared by CLI and (later) server
- [x] `locus` CLI: `verify`, `policy check`, `evidence emit`, `contribution`, `vouch check`
- [x] Normative spec + JSON Schemas + conformance suite
- [x] This repository gated by its own policy
- [ ] Sigstore signing envelope for evidence
- [x] `VOUCHED.td` reader for the `vouched` tier
- [ ] **Gate-gaming resistance** — privileged check paths + `coverage-delta`
      ([ADR 0006](adr/0006-the-gate-must-resist-what-it-gates.md)). This closes a
      hole that exists *today*: `tests: pass` binds even when the tests changed in
      the same contribution. Honest evidence, false conclusion.
- [ ] **Actor identity as a delegation chain**
      ([ADR 0007](adr/0007-actor-identity-is-a-delegation-chain.md)). Cheap now;
      the cost grows with every hour of trust history recorded under the flat shape.
- [ ] Authorship records
      ([ADR 0008](adr/0008-authorship-is-declared-not-detected.md))

**Done when** a maintainer outside this project can adopt the CLI in their own CI
and get a useful verdict.

`locus contribution` was the missing half of that: until it existed, adopting
GitLocus meant hand-writing the contribution document, and nobody adopts a
format they have to author by hand. This repository's own gate now builds its
contribution with the shipped command rather than a script that only exists
here — whatever we need in order to adopt GitLocus, an outside repository needs
too.

Still outstanding before this stage is honestly done: **evidence is not signed.**
It is structurally validated and bound to a revision, but nothing proves who
produced it. That is the next piece of work, not a detail.

## Stage 1 — The gate as a service

A GitHub App and a hosted `locusd`, so adoption does not require wiring CI by hand.

- Evidence ingestion from Actions
- Policy evaluation as a check run, with the verdict rendered in the PR
- OpenAPI 3.1 contract, written before the server
- **MCP server, promoted in priority.** Provenance exists only at the moment of
  production and the harness is the only thing present for it — so the harness,
  not the forge, is the highest-value integration surface. Agents are evidence
  *producers*, not only subjects. See
  [AGENT-ERA §2](AGENT-ERA.md).
- Verdict caching by content hash. Free consequence of `evaluate` being pure, and
  what makes swarm-scale evaluation affordable rather than merely correct.

**Constraint discovered during planning:** the target VPS (`clawguard`) has 2
cores and 3.8 GiB RAM with existing workloads. Adequate for a pilot; not the scale
target. Capacity decision belongs at the start of this stage, not the end.

**Done when** a third-party repository installs the App and gets verdicts without
writing a workflow.

## Stage 2 — Attention allocation

The part that is actually the product: the ranked queue.

- Web UI (TypeScript/React) generated against the Stage 1 OpenAPI contract
- **The licence-integrity ledger** — what share of this codebase carries a human
  authorship claim, who made it, and where the gaps are. The screen that speaks to
  a solo developer ("is this mine?") and a CTO ("prove it across ten thousand
  repositories") with one view.
- Contributions ordered by what a maintainer can act on
- Evidence rendered by class, so deterministic and assessed never look alike
- Trust graph, deriving tiers from evidence history and `VOUCHED.td`, with scope
  attenuation enforced ([ADR 0007](adr/0007-actor-identity-is-a-delegation-chain.md))

**Done when** a maintainer with a hundred open contributions can tell in under a
minute which five to look at.

## Stage 3 — First-party evidence

Until here, GitLocus only *consumes* evidence others produce. This stage produces
it.

- Ephemeral workspaces that execute and observe
- Deterministic evidence generated under a known environment
- Compute-metered economics — the first point at which billing has anything to
  measure

**Why not sooner:** producing evidence is expensive and commoditised. Consuming
and ranking it is neither. Doing this first would have spent the entire budget on
sandboxing.

## Stage 4 — Native git surface *(conditional)*

Only if owning the storage demonstrably unlocks something the API layer cannot do.

The condition is real. If Stages 1–3 deliver the value without it, this stage
never gets built, and that is the correct outcome rather than a shortfall. See
[ADR 0001](adr/0001-evidence-not-a-forge.md).

---

## Open questions

Tracked in [`spec/README.md` §8](../spec/README.md) — signing, blast radius,
evidence expiry, cross-repository trust, and whether the predicate is registered
with in-toto or namespaced under SLSA.

Named as unsolved in [`AGENT-ERA.md`](AGENT-ERA.md), and more important than most
of the above:

- **Swarm cost.** Standing limits what a swarm can *bind*; nothing limits what it
  can *consume*. Rate and priority tied to tier is the likely shape, undesigned.
- **Declaration granularity.** A single authorship checkbox over fifty files is a
  rubber stamp, and gets more so as models improve.
- **Prompt injection against agents with write access.** Class separation bounds
  review; it does nothing for an agent that can push.
- **Trust bootstrapping.** Tiers exist. How an identity earns promotion does not.

Feedback on any of these is more valuable right now than code.
