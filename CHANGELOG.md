<!-- SPDX-License-Identifier: Apache-2.0 -->
# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Pre-1.0, and the spec is a draft.** Anything in `spec/` may break in a patch
release until v1. That is stated in the spec itself and it is not a formality.

## [Unreleased]

### Added

- ADR 0011 — the merge decision is the kernel of a git platform, and it is built
  first because it is the half that cannot be commoditised. Supersedes ADR 0001,
  keeping its finding that building the hosting half first spends the budget on
  the commodity half.
- ADR 0012 — provenance exists only at the moment of production, so the harness,
  not the forge, is the integration surface that matters.
- A release check that runs each built binary and refuses to ship it unless
  `locus --version` reports the tag being released.

### Changed

- The `main` branch ruleset no longer has a bypass. Direct pushes to `main` are
  impossible for everyone including the owner, twelve deterministic checks are
  required, and required approvals are zero — see the README for why that is the
  honest configuration for a single-maintainer repository rather than a
  weakening.
- The README opens on what this converges rather than on what it is not.

### Fixed

- The workspace version said `0.0.1` while `v0.0.2` was the published release, so
  every binary in that release reported the wrong version. Corrected to `0.0.2`,
  and the release now fails rather than shipping the mismatch again.
- `THREAT-MODEL.md` and `.github/CODEOWNERS` both claimed the branch ruleset
  restricts pushes to privileged paths to code owners. It carries no path
  restriction; CODEOWNERS routes review.

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

[Unreleased]: https://github.com/hey-vera/gitlocus/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/hey-vera/gitlocus/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/hey-vera/gitlocus/releases/tag/v0.0.1
[#1]: https://github.com/hey-vera/gitlocus/pull/1
[#2]: https://github.com/hey-vera/gitlocus/pull/2
[#3]: https://github.com/hey-vera/gitlocus/pull/3
[#4]: https://github.com/hey-vera/gitlocus/pull/4
[#5]: https://github.com/hey-vera/gitlocus/pull/5
[#6]: https://github.com/hey-vera/gitlocus/pull/6
