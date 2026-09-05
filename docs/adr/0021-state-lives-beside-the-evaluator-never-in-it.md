<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0021 — State lives beside the evaluator, never in it

- **Status:** accepted
- **Date:** 2026-09-05
- **Amends:** [0016](0016-locusd-lives-here-under-agpl.md) — the licence table gains a second AGPL crate

## Context

Invariant 4 in [AGENTS.md](../../AGENTS.md): verdicts are pure — no clock, no
network, no ambient state, no dependence on evidence order — and there is a
property test for it. `locusd` was built to that standard and says so in its own
module documentation: it stores nothing, reads no clock, makes no network call,
and needs no authentication because there is nothing to protect. The same
documentation anticipated this record: *when a ranked queue needs history it will
need a store and an identity model, and that is a different service with a
different threat model.*

[0020](0020-identity-is-federated-standing-and-delegation-are-native.md) brings
the store and the identity model: principals, bindings, grants, an issuer key,
recorded verdicts. Every one of those is state, and the question is where it lives
so that the evaluator stays what it is — not by convention, which erodes, but by
construction, which does not.

## Decision

**Two processes, one boundary.**

`locusd` stays exactly what it is: the pure evaluator over HTTP — `/v0/verdict`,
`/healthz`, its contract — unauthenticated, stateless, with no store, no key and no
outbound network. Its systemd unit already says it reads nothing, writes nothing
and calls nothing; that stays true.

A new crate, `crates/locus-ledger`, AGPL-3.0-only like `locusd`, builds the
`locus-ledger` binary: the principal registry, the grant ledger and token issuer,
the evidence ledger, and the authenticated API under `/v1/`. It depends on
`gitlocus-core` and calls `evaluate` in-process, so a verdict it records and a
verdict from `locusd` or the CLI come from the same compiled function
([0004](0004-rust-core-shared-by-cli-and-server.md)). It does not depend on
`locusd`, and `locusd` does not depend on it.

Both sit behind the one origin at locus.heyvera.org, routed by path; the console's
content-security policy stays `connect-src 'self'`.

**The proof is the boundary, not a promise.** A store cannot leak into the
evaluator because the evaluator's binary has no store in it. The issuer key cannot
be read by the public compute endpoint because that endpoint runs in a different
address space. Neither claim needs a test to stay true; a change that violated
either would be a change to a Cargo manifest and a systemd unit, which is visible
in review in a way a function call is not.

**The ledger records what it verified.** A signer on an evidence record is a
conclusion the ledger reached by validating a credential (spec §3.3.1). A third
party who does not trust the ledger must be able to reach the same conclusion, so
the ledger keeps the credential it validated alongside the record it produced —
expired by the time anyone reads it, and therefore harmless to keep — and signs the
bundle of contribution, evidence and verdict with its own key.

## Consequences

**Two units, two deploys, two health checks.** #80 already says deployment is the
last command that never moved into the `justfile`; it now has two binaries to
move, and `just brief` compares two live versions with the release instead of one.

**A store with migrations.** SQLite on the single node is enough for a pilot and
is chosen for having nothing to operate. The schema is the durable asset 0020
warns about; migrations are forward-only and tested from an empty database and
from the previous shape.

**The ledger is a trusted verifier.** Recording what it verified narrows the trust
to "the ledger did not lie about what it saw", which a recorded credential lets
anyone check. It does not remove it. That is the same shape as every other signed
producer in this design, stated here rather than implied away.

**A bigger AGPL side, and a second place the licence line can be crossed.** 0016's
table, `deny.toml`'s AGPL scope and `just licence-headers` all gain the new path;
the header check fails a file in the wrong place, and the dependency direction —
the core never depends on either server crate — stays a review concern, as 0016
already records.

**Resource cost.** Two small Rust processes on two cores is not a concern today.
If it becomes one, the answer is a bigger host, not merging the processes.

## Alternatives rejected

**One process, a module boundary.** Simplest to deploy. Rejected because the issuer
key and the store would share an address space with a public, unauthenticated
compute endpoint that parses attacker-chosen documents. The totality properties in
`crates/locusd/tests/properties.rs` make a panic there unlikely; unlikely is not the
standard this project applies to its one security-relevant decision.

**State inside `gitlocus-core`.** 0004's constraint — the core must stay free of
I/O — is what keeps determinism testable. Unchanged.

**A separate repository for the ledger.** 0016's argument holds: the repository
layout is a two-way door, and a change spanning the kernel, the evaluator and the
ledger wants one pull request and one CI run.

**A hosted database.** Nothing to migrate to yet, a network dependency in the write
path, and a second party holding the durable asset. Revisit when there is a second
node.
