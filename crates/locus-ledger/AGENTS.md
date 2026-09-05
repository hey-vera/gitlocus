<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# AGENTS.md — crates/locus-ledger

The repository-wide instructions in [`../../AGENTS.md`](../../AGENTS.md) apply
here too, and so does everything [`../locusd/AGENTS.md`](../locusd/AGENTS.md)
says about the licence: **this crate is AGPL-3.0-only**, every file starts with
`// SPDX-License-Identifier: AGPL-3.0-only`, `just licence-headers` enforces it,
and nothing moves from here into `gitlocus-core` or `gitlocus-cli` without a
licence decision first.

What is different here, and why this file exists:

**This crate holds state, and the evaluator must never see it.**
[ADR 0021](../../docs/adr/0021-state-lives-beside-the-evaluator-never-in-it.md)
puts principals, grants, the issuer key and recorded verdicts in this process
and keeps `locusd` a pure evaluator in another. The boundary is the dependency
graph and the process, not a convention: this crate depends on `gitlocus-core`
and calls `evaluate` in-process; `locusd` does not depend on this crate, and this
crate does not depend on `locusd`. A change that makes either true has crossed
the line ADR 0021 drew, whatever else it does.

**The store is the one asset that cannot be reconstructed.** Migrations are
forward-only. A database written by a newer schema is refused, never silently
read. A test opens an empty database and one at the previous shape before any
migration lands.

**Nothing here may raise a tier.** The chain this crate constructs for the
evaluator carries ceilings; `Actor::effective_tier` in the core takes the
minimum. A path that let anything in this crate — a grant, a payment, a
binding — reach `Actor::tier` upward would be the Sybil failure #73 and #74 name,
arriving through the service.
