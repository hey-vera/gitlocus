<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0001 — Build the evidence layer, not a forge

- **Status:** accepted
- **Date:** 2026-08-18

## Context

The founding brief for this project described "GitHub for humans and autonomous
agents" — an agent-native forge, with repositories, workspaces, sandboxes,
structural merge and its own hosting.

Two things make that the wrong first move.

**GitHub is already shipping into that lane.** Agent HQ launched in November 2025:
multi-vendor agents under one control plane with enterprise governance. Agentic
Workflows entered technical preview in February 2026 — markdown workflows compiled
to Actions, read-only by default, with Claude Code, Codex and Gemini CLI as built-in
engines. Artifact attestations, immutable releases, rulesets, merge queue and
Dependabot cooldown all shipped in the same window. Competing on "run agents near
the repository" means racing the incumbent on the feature they are actively
building.

**The actual problem is not hosting.** curl ended its bug bounty in January 2026
not because its git server was inadequate, but because evaluating submissions cost
more than producing them. Ghostty closes drive-by AI pull requests unread for the
same reason. Neither problem is touched by owning the repository storage.

## Decision

Build the **evidence, policy and trust layer**. Treat the forge as a surface that
consumes it.

The layer is deliberately forge-external:

- A Contribution is identified by content digests, not by a pull-request number.
- A Policy lives in the repository as a file, readable by anything that can read
  the repository.
- Evidence is an in-toto statement, transportable anywhere.
- A Verdict is a pure function of the three.

None of that requires owning the git storage.

## Consequences

**Good.** Buildable by a small team. Adoptable one repository at a time with no
migration. Useful on GitHub today and on anything else later. Composes with SLSA
Source Track — which explicitly leaves the evidence format to the source-control
system — instead of competing with it.

**Bad.** We do not control the surfaces where the verdict is displayed, so the UX
ceiling is set by what a check run and a comment can express. We depend on the
forge to report changed paths honestly. And "a policy layer" is a harder story to
tell than "a new GitHub".

**Deferred, not refused.** A native git surface stays on the roadmap as Stage 4,
conditional on owning the storage unlocking something the API layer demonstrably
cannot do. If that condition is never met, the stage is never built, and that is a
success rather than a failure.

## Alternatives rejected

**Build the forge first, add evidence later.** This produces exactly the
anti-pattern the founding brief itself warned against: a prettier GitHub with AI
attached. It also spends the entire budget on the commodity half.

**Spec only, no implementation.** A specification with no running code gets no
feedback and reads as vaporware. The reference implementation and the conformance
suite are what make the spec falsifiable.
