<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0004 — One Rust core, shared by the CLI and the server

- **Status:** accepted
- **Date:** 2026-08-18

## Context

Policy evaluation will eventually run in three places: a contributor's laptop, a
CI job, and a hosted service. The classic outcome is three implementations that
agree until they do not — and the disagreement surfaces as "it passes locally but
fails in CI", which trains everyone to stop trusting the local run.

Language choice was genuinely contested. The entire provenance ecosystem —
`sigstore-go`, `in-toto-golang`, `slsa-verifier`, `cosign`, `gh attestation` — is
written in Go, and interoperability bugs are this project's largest technical
risk. Against that: Rust is the existing language across this organisation's other
systems, and the eventual server is a long-lived, high-concurrency,
crypto-adjacent process, which is what Rust is good at.

## Decision

**One core crate, `gitlocus-core`**, containing the model and the evaluator. The CLI
depends on it. The future server will depend on the same crate at the same
version. There is exactly one implementation of `evaluate`.

**Rust** for the core, CLI and server. **TypeScript and React** for the web
interface when it arrives.

**Sigstore and in-toto operations shell out to the Go binaries** (`cosign`,
`gh attestation`, `slsa-verifier`), pinned by digest, rather than being
reimplemented in Rust. This takes the ecosystem's battle-tested implementations
instead of betting v0 on `sigstore-rs` maturity, and confines the interop risk to
a process boundary we control.

**The API contract is OpenAPI 3.1, written before the server.** The web client is
generated from it. This is the only way "the human interface uses the same API as
agents" survives contact with a deadline.

## Consequences

**Good.** A local verdict and a CI verdict cannot diverge — not by convention but
because it is the same compiled code. This is a correctness property, and there is
a test asserting it. Single static binary for deployment. No crypto reimplemented.

**Bad.** Shelling out means the CLI is not fully self-contained: signing needs
`cosign` on PATH. That is a real distribution cost, accepted deliberately, and
revisitable once the evidence format stabilises. Rust also means the project is
less immediately approachable to the Go-centric supply-chain community that would
otherwise be natural contributors.

**Constraint.** The core must stay free of I/O. Evaluation is a pure function;
anything that reads a file, a clock or a socket belongs in the CLI or the server,
never in `gitlocus-core`. This is what keeps determinism testable.

## Alternatives rejected

**Go throughout.** Better library access and a friendlier landing in the
supply-chain community. Rejected on organisational fluency: every other system
here is Rust, and a second language for a small team costs more than the library
convenience returns.

**TypeScript.** Natural for GitHub Actions distribution, weakest for a
signed-attestation CLI and a long-lived server.

**Separate implementations per surface.** The failure mode this decision exists to
prevent.
