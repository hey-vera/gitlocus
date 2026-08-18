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
- [x] `locus` CLI: `verify`, `policy check`, `evidence emit`
- [x] Normative spec + JSON Schemas + conformance suite
- [x] This repository gated by its own policy
- [ ] Sigstore signing envelope for evidence
- [ ] `VOUCHED.td` reader for the `vouched` tier

**Done when** a maintainer outside this project can adopt the CLI in their own CI
and get a useful verdict.

## Stage 1 — The gate as a service

A GitHub App and a hosted `locusd`, so adoption does not require wiring CI by hand.

- Evidence ingestion from Actions
- Policy evaluation as a check run, with the verdict rendered in the PR
- OpenAPI 3.1 contract, written before the server
- MCP server over the same API, so agents reach it the way agents actually work

**Constraint discovered during planning:** the target VPS (`clawguard`) has 2
cores and 3.8 GiB RAM with existing workloads. Adequate for a pilot; not the scale
target. Capacity decision belongs at the start of this stage, not the end.

**Done when** a third-party repository installs the App and gets verdicts without
writing a workflow.

## Stage 2 — Attention allocation

The part that is actually the product: the ranked queue.

- Web UI (TypeScript/React) generated against the Stage 1 OpenAPI contract
- Contributions ordered by what a maintainer can act on
- Evidence rendered by class, so deterministic and assessed never look alike
- Trust graph, deriving tiers from evidence history and `VOUCHED.td`

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

Feedback on any of these is more valuable right now than code.
