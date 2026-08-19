<!-- SPDX-License-Identifier: Apache-2.0 -->
# GitLocus

[![CI](https://github.com/hey-vera/gitlocus/actions/workflows/ci.yml/badge.svg)](https://github.com/hey-vera/gitlocus/actions/workflows/ci.yml)
[![Supply chain](https://github.com/hey-vera/gitlocus/actions/workflows/supply-chain.yml/badge.svg)](https://github.com/hey-vera/gitlocus/actions/workflows/supply-chain.yml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/hey-vera/gitlocus/badge)](https://scorecard.dev/viewer/?uri=github.com/hey-vera/gitlocus)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Contribution provenance and policy for repositories in the agent era.**

Producing a change is now cheap. Deciding whether to accept one is not. GitLocus
is a signed record of *how a contribution was produced*, a policy the repository
owns that says what such a record must contain, and a gate that turns a pile of
incoming work into a ranked queue with the evidence attached.

It runs on top of the forge you already use. It is not a GitHub replacement.

> **Status: v0.0.2, pre-release.** The spec is a draft and will break before v1.
> The gate works and runs on this repository's own pull requests. Binaries carry
> signed build provenance. Signature *verification* is implemented; the signing
> *producer* is not, so a `signed_by` rule currently refuses everything — which is
> why this repository does not yet use one.

---

## The problem

In January 2026 curl ended its bug bounty after six years, 87 valid findings and
over $100,000 paid. The valid-report rate had fallen from above 15% to under 5%
while submission volume climbed. Ghostty now closes drive-by AI pull requests
unread and requires first-time contributors to be vouched for.

Neither project has a quality problem with AI. They have an **evaluation** problem:
the cost of producing a plausible contribution collapsed, and the cost of checking
one did not. Maintainer attention is the scarce resource, and nothing in the
existing toolchain treats it that way.

The instinct is to detect and reject AI-authored code. That does not work — final
source text carries no reliable signal of how it was written, and it never will.
The tractable question is not *who wrote this* but **what can be proven about it,
and who is answerable for it.**

## The model

Five types, and no more until something forces a sixth.

| | |
|---|---|
| **Actor** | Who produced the change — a human, an agent, or an **operator+agent pair**. Key-bound, never self-declared. |
| **Contribution** | The proposed change, identified by content digests. A GitHub PR, a GitLab MR and a mailed patch series are the same object. |
| **Evidence** | A signed claim about a contribution, carrying the class that says what it is worth. |
| **Policy** | The repository's own rules, versioned in the repository. |
| **Verdict** | Reproducible output: what is still missing, and where this sits in the queue. |

### Evidence has classes, and they are not interchangeable

This is the idea the whole system turns on.

- **Deterministic** — reproducible by a third party with the same inputs. Exit
  codes, digests, test results. **Only this class can satisfy a requirement.**
- **Assessed** — a heuristic or model judgement. Surfaced next to the verdict,
  binding on nothing, however confident it sounds.
- **Attested** — a human took responsibility. Cannot be produced by automation.

A passing test suite and a language model saying "looks fine" are both claims about
a change. Collapsing them into one confidence score is how review budget gets spent
on the wrong pull request. GitLocus keeps them apart in the type system, not just
in the UI.

### The operator+agent pair

An agent acting on a person's instruction is neither purely the agent nor purely
the person: the agent did the work, and the person is answerable for it. Systems
that force this into one identity lose whichever half they discard. GitLocus
records both, and an agent that nobody will answer for does not clear a meaningful
trust tier.

## Try it

```bash
cargo build --release
```

**1. Describe the change.** `locus` reads the facts out of git — you never write
this document by hand:

```bash
./target/release/locus contribution --base main --head HEAD > contribution.json
```

Add `--agent claude-code --operator you@example.com` when an agent did the work
and you are answerable for it. That is the operator+agent pair, recorded rather
than flattened.

**2. Record what ran.** One evidence record per check:

```bash
./target/release/locus evidence emit --kind tests --class deterministic --outcome pass \
  --subject "$(git rev-parse HEAD)" --produced-by local --produced-at "$(date -u +%FT%TZ)"
```

**3. Ask what it still needs:**

```bash
./target/release/locus verify --contribution contribution.json --evidence evidence.json
```

The exit code is zero only when the policy is satisfied. The same evaluation runs
here and in CI out of the same crate, so a local verdict and the gate's verdict
cannot disagree — if they ever do, that is a bug in this project.

**Trust files.** If your project already has a `VOUCHED.td`, GitLocus reads it:

```bash
./target/release/locus vouch check --user someone
./target/release/locus contribution --base main --head HEAD --vouched-file VOUCHED.td
```

A vouch raises an unknown actor to `vouched`. A denouncement caps the tier no
matter what else was supplied, and says so on stderr — a downgrade nobody
notices would be a bug.

## Verifying a release

Release binaries carry a signed SLSA build provenance attestation:

```bash
gh attestation verify locus-x86_64-unknown-linux-gnu --repo hey-vera/gitlocus
```

That is the point of shipping them this way: a provenance tool distributing
unattested binaries would refute itself.

## How this repository is built

The CI is deliberately structured as the same three classes the spec defines.
Deterministic checks block. Human approval is required by a branch ruleset rather
than by a workflow — a workflow could be edited in the same pull request it
governs.

No assessed producer is wired up yet: the verdict renders advisory findings and
the format carries them, but a model reviewer arrives in Stage 1. Saying so
matters more than the story being tidy.

GitLocus's own policy lives in [`.gitlocus/policy.yml`](.gitlocus/policy.yml) and
runs against every pull request here. A policy engine whose authors exempt
themselves is not evidence of anything.

## What this is not

- Not an AI-code detector. See [ADR 0002](docs/adr/0002-no-ai-authorship-detection.md).
- Not a git host. See [ADR 0001](docs/adr/0001-evidence-not-a-forge.md).
- Not a replacement for CI, Sigstore, SLSA, or Vouch — it composes with all four.
- Not a reputation score. Standing is derived from evidence history; a stored
  number is a number to farm.

Full list: [docs/NON-GOALS.md](docs/NON-GOALS.md).

## Prior art this builds on rather than replaces

- **[SLSA Source Track](https://slsa.dev/spec/v1.2/source-requirements)** (v1.2,
  approved Nov 2025) defines Source Provenance Attestations and leaves the evidence
  format to the source-control system. GitLocus aims to be one such format.
- **[in-toto](https://github.com/in-toto/attestation)** provides the attestation
  envelope. There is still no registered predicate for contribution or agent
  decisions ([in-toto/attestation#554](https://github.com/in-toto/attestation/issues/554)).
- **[Vouch](https://github.com/mitchellh/vouch)** already solved the social half.
  GitLocus reads `VOUCHED.td` rather than defining a competing trust file.

## Documentation

| | |
|---|---|
| [spec/](spec/) | The normative model and JSON Schemas |
| [docs/adr/](docs/adr/) | Why things are the way they are |
| [issues](https://github.com/hey-vera/gitlocus/issues) · [milestones](https://github.com/hey-vera/gitlocus/milestones) | What is being worked on, and the stages |
| [docs/AGENT-ERA.md](docs/AGENT-ERA.md) | The problems this is an answer to — swarms, harnesses, and models that do not exist yet — and which remain unsolved |
| [THREAT-MODEL.md](THREAT-MODEL.md) | What this defends against, and what it does not |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Including the AI-contribution policy |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability |
| [AGENTS.md](AGENTS.md) | Instructions for coding agents |

## Licence

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
