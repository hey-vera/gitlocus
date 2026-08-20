<!-- SPDX-License-Identifier: Apache-2.0 -->
# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Pre-1.0, and the spec is a draft.** Anything in `spec/` may break in a patch
release until v1. That is stated in the spec itself and it is not a formality.

## [Unreleased]

### Added

- **One definition of every check.** A `justfile` defines each check once, with a
  recipe named for the status-check context it satisfies, and the workflows
  invoke those recipes rather than restating the commands.
  `.github/required-checks.txt` records the mapping, including which checks
  cannot run in full outside GitHub and why. ADR 0017.
- **The documentation is under test.** `crates/repo-conformance` asserts that
  relative links resolve, that the paths this project's own documents name
  exist, that every required check has a job and a recipe, that every decision
  record states what it costs and that supersessions link both ways, that one
  version string means one version, that every learning names its guard, and
  that a crate meant for crates.io carries what crates.io renders.
- **`LEARNINGS.md`**, carrying the record of claims this project shipped
  stronger than the implementation, and the traps that cost real time. Every
  entry names the check that would catch a recurrence, an issue, or an explicit
  `none` with the reason.
- **`just brief`**, which prints where the project is — issues by milestone, the
  latest release, whether the service is answering, and whether the settings the
  record depends on are still set. It reads GitHub live and is never committed,
  so it cannot go stale.
- **Properties for the invariants that are quantified over all inputs.** Seven in
  `gitlocus-core` and four in `locusd`, including the totality contracts for the
  vouch and policy parsers and for the public HTTP surface. ADR 0018.
- **An attested bill of materials on every release.** SPDX generated from the
  checkout and attested under the same workflow identity as the build
  provenance, with both bundles covered by `SHA256SUMS`.
- **Documentation that builds.** `just doc` denies broken intra-doc links, and
  `just lint` depends on it. Nothing built the docs at all before.
- **Both published crates are publishable** — README, keywords, categories and
  docs.rs metadata, verified with `cargo publish --dry-run`.
- **Immutable releases are enabled**, from the next tag onwards. The claim that
  they were had stood since the first release.
- **Authorship declarations.** An `attested` Evidence record may carry a claim of
  `human`, `directed_agent`, `generated`, or `derived` with its source, and a
  policy may say `authorship: [human, directed_agent]` to refuse generated work.
  Silence is read as `generated`, so a contribution that declares nothing fails
  such a rule — any weaker default would let undeclared work quietly claim
  copyright. `locus authorship declare` emits the record. Specification §3.3.2,
  conformance clause 10, ADR 0008.
- **A composite action**, so adoption is five lines and needs no Rust toolchain.
  It verifies the released binary's attestation before executing it.

### Fixed

- **A verdict depended on the order of the evidence array.** The `advisory` list
  was emitted in arrival order, so two `assessed` records arriving in a different
  order produced different verdict bytes for the same inputs. `decision` was
  never affected, but the verdict was not byte-identical — which is what
  specification §6 clause 5 requires and what makes a verdict
  content-addressable. `advisory` is now ordered and deduplicated, and §3.5 says
  so. Found by a property test on its first run; the conformance test covering
  clause 5 had used a single `assessed` record, where the order cannot vary.
- **`AGENTS.md` claimed its four documented commands were the ones CI ran.** CI
  passed `--locked` on all four and ran nine more checks besides, so a change
  could be green locally and red on eleven required checks.
- **`cargo test --doc` reported `running 0 tests`** under a comment saying it
  kept the model's documentation examples honest. Every fenced block in a doc
  comment is `yaml` or `text`, which rustdoc does not compile. The examples are
  now complete policies checked by a test, and the README's Rust example is a
  real doctest.
- The evidence collector read a check run's `status` before its `conclusion`.
  GitHub reports `in_progress` alongside `conclusion: success` and holds it for
  minutes, so a check that had *passed* was recorded `inconclusive` and blocked a
  contribution every check agreed with. A conclusion is authoritative.
- The collector's exclusion list was hard-coded to this repository's job names,
  so an adopter's own gate job would wait on itself until the retry budget ran
  out. It now comes from the environment.

## [0.0.3] — 2026-08-19

### Added

- **A contribution is governed by the policy at its base revision as well as the
  one it ships.** Previously the gate read `.gitlocus/policy.yml` out of the pull
  request's own tree, so a change deleting every rule was judged by a document
  with no rules in it and came back `satisfied` with no evidence and no standing.
  `locus verify` takes `--governing-policy`, repeatable; rules from the base are
  reported prefixed `governing:`. ADR 0013.
- **The gate is built from the base revision**, so the evaluator is no longer
  supplied by its subject. Together with the above: a contribution cannot
  influence how it is judged. ADR 0014.
- **A mutation check over the changed lines** (`cargo mutants --in-diff`),
  required by this repository's own policy. Coverage cannot see a loosened
  assertion; a surviving mutant can. ADR 0015 amends ADR 0006, whose two
  prescribed mechanisms turned out to be inapplicable and overstated
  respectively.
- **An `msrv` job** that reads `rust-version` out of the workspace manifest and
  compiles against it, and a **`platforms` matrix** covering all three released
  targets on native runners.
- ADR 0011 — the merge decision is the kernel of a git platform, and it is built
  first because it is the half that cannot be commoditised. Supersedes ADR 0001,
  keeping its finding that building the hosting half first spends the budget on
  the commodity half.
- ADR 0012 — provenance exists only at the moment of production, so the harness,
  not the forge, is the integration surface that matters.
- A release check that runs each built binary and refuses to ship it unless
  `locus --version` reports the tag being released.
- Conformance tests for specification clauses 4 and 6, which had none. Every
  clause 1 through 9 now has an executable form.
- Sixteen integration tests driving the `locus` binary. It previously had one
  test, checking only that clap's definition was well formed.

### Changed

- The `main` branch ruleset no longer has a bypass. Direct pushes to `main` are
  impossible for everyone including the owner, fifteen deterministic checks are
  required, and required approvals are zero — see the README for why that is the
  honest configuration for a single-maintainer repository rather than a
  weakening. Every merge before this logged `result=bypass`, meaning no rule in
  that ruleset had ever been evaluated.
- The README opens on what this converges rather than on what it is not.
- `THREAT-MODEL.md` gains an entry for rule deletion, and every entry now names
  the test that backs it.

### Fixed

- The workspace version said `0.0.1` while `v0.0.2` was the published release, so
  every binary in that release reported the wrong version. Corrected to `0.0.2`,
  and the release now fails rather than shipping the mismatch again.
- `THREAT-MODEL.md` and `.github/CODEOWNERS` both claimed the branch ruleset
  restricts pushes to privileged paths to code owners. It carries no path
  restriction; CODEOWNERS routes review.
- `Contribution::id` could return an empty string and `PolicyError`'s `Display`
  could render nothing, neither of which any test noticed. Both found by
  mutation testing.
- `crates/gitlocus-cli` had no behavioural tests: 24 surviving mutants, including
  `changed_paths -> Ok(vec![])`, which makes every verdict `satisfied`, and
  `verify -> Ok(Default::default())`, which makes the exit code unconditionally
  zero. Now zero survivors.

### Removed

- `docs/NON-GOALS.md` and `docs/AGENT-ERA.md`. Both restated decisions the ADRs
  own, and both asserted that hosting, CI and issue tracking were out of scope,
  which ADR 0011 reverses.

## [0.0.2] — 2026-08-18

### Fixed

- Windows release builds ran under PowerShell rather than bash, where `"$TARGET"`
  expands to nothing and cargo reports an empty target. `shell: bash` is now a job
  default so a step added later cannot reintroduce it. ([#6])

### Known issue

- Binaries in this release report version `0.0.1`. The workspace version was never
  bumped and nothing checked it. Fixed in the next release.

## [0.0.1] — 2026-08-18

First tagged release. The model, the specification, the reference implementation
and a gate that runs on this repository's own pull requests.

### Added

- The five-type model — Actor, Contribution, Evidence, Policy, Verdict — with
  evidence classes enforced in the type system: `deterministic` may satisfy a
  requirement, `assessed` never may, `attested` covers approvals only.
- `spec/` — the normative model, four JSON Schemas, and a conformance suite that
  is the executable form of the specification's conformance section.
- `locus` — `verify`, `policy check`, `evidence emit`, `contribution` built from
  git, and `vouch check`.
- Signature constraints: `signed_by` per check and `approvals_signed_by`, with
  `Evidence::signer` never read from input, so a signer is a conclusion the
  verifier reaches rather than a claim a document makes. ([#4], [#5])
- `VOUCHED.td` support in the format used by mitchellh/vouch, rather than a
  competing trust file. ([#2])
- ADRs 0001–0010. ([#3])
- CI, supply-chain auditing, OpenSSF Scorecard and licence badges. ([#1])

[Unreleased]: https://github.com/hey-vera/gitlocus/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/hey-vera/gitlocus/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/hey-vera/gitlocus/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/hey-vera/gitlocus/releases/tag/v0.0.1
[#1]: https://github.com/hey-vera/gitlocus/pull/1
[#2]: https://github.com/hey-vera/gitlocus/pull/2
[#3]: https://github.com/hey-vera/gitlocus/pull/3
[#4]: https://github.com/hey-vera/gitlocus/pull/4
[#5]: https://github.com/hey-vera/gitlocus/pull/5
[#6]: https://github.com/hey-vera/gitlocus/pull/6
