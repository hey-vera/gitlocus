<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0016 — locusd lives in this repository, under AGPL-3.0

- **Status:** accepted
- **Date:** 2026-08-19
- **Amended by:** [0021](0021-state-lives-beside-the-evaluator-never-in-it.md) — a second AGPL crate, `crates/locus-ledger`, joins the table below
- **Supersedes the licensing and repository decisions recorded in** [#13](https://github.com/hey-vera/gitlocus/issues/13)

## Context

Issue #13 recorded two decisions together: `locusd` would live in a separate
repository, and it would be AGPL-3.0. Both were taken when the server was an
adoption convenience for a forge-external tool — something you added to make
GitLocus easier to install.

[ADR 0011](0011-the-kernel-of-a-git-platform.md) changed that premise. The server
is the spine of the platform, not an accessory to a CLI. A boundary that was
cheap when it separated a tool from its optional daemon now runs through the
middle of one product.

## Decision

**One repository. The licence is split by directory, not the code by repository.**

| path | licence |
|---|---|
| `spec/`, `crates/gitlocus-core`, `crates/gitlocus-cli` | Apache-2.0 |
| `crates/locusd` | AGPL-3.0-only |
| `crates/locus-ledger` (planned, ADR 0021) | AGPL-3.0-only |

The reasoning is an asymmetry between the two halves of the original decision:

**The licence is a one-way door.** Starting Apache-2.0 and wishing you had
started AGPL is unrecoverable the moment an outside contributor lands server
code, because relicensing then needs their consent — and this project
deliberately chose the DCO over a CLA
([ADR 0008](0008-authorship-is-declared-not-detected.md)), so there is no
assignment to fall back on.

**The repository layout is a two-way door.** Splitting one repository into two is
a subtree operation that preserves history, and merging two into one is the same
operation backwards. Nothing is lost by choosing the layout that suits the work
today.

So the irreversible choice is made conservatively and the reversible one is made
for velocity. That is the whole argument.

**The dependency direction works and the reverse would not.** Apache-2.0 is
one-way compatible with the GPLv3 family, so an AGPL `locusd` may depend on an
Apache-2.0 `gitlocus-core`. A permissively licensed kernel with a copyleft server
on top is the only arrangement of these two licences that composes at all.

**Not FSL or BUSL.** Source-available is not open source, and shipping it would
be self-refuting for a project whose flagship application is keeping open-source
licences enforceable. That part of #13 was right and is unchanged.

## Consequences

**Good.** A change spanning the kernel and the server is one pull request with
one CI run and one verdict. The conformance suite sits next to its largest
consumer, so the thing most likely to break the contract is the thing best placed
to notice. And the format stays Apache-2.0, so another implementation of the
specification needs no contact with copyleft code — which is the entire reason
the split existed.

**Bad — contributors face two licences in one tree.** That is real friction and
it is the strongest argument for the separate repository. It is mitigated by
making the map executable rather than documentary: the `licence-headers` check
now enforces which identifier belongs in which directory, so a file in the wrong
place fails CI instead of being noticed in review, or not.

**Bad — the boundary is now a convention rather than a repository.** Nothing
physically stops someone adding a `crates/locusd` dependency to `gitlocus-core`
and pulling AGPL into the kernel. `cargo-deny` and the header check catch the
licence half; the dependency direction is a review concern until something
enforces it, and that is written down here rather than assumed.

**Neutral — publishing.** Crates declare their own `license` field, so a mixed
workspace publishes to crates.io without ceremony. `locusd` is a binary and is
not intended for crates.io at all.

## Alternatives rejected

**Separate repository, AGPL-3.0, as #13 recorded.** The cleanest licence story
and the sharpest boundary. Rejected because the ongoing cost lands exactly where
the work is: two pull requests in two repositories with no atomic CI, for changes
that span a kernel and its only serious consumer. It remains available at any
time, which is the point of choosing the reversible option.

**Apache-2.0 throughout.** Simplest for contributors and fastest to start.
Rejected because it spends the one-way door for a convenience: a hosted
competitor could take the server closed, and the decision cannot be walked back
once anyone else has contributed to it.

**Decide later, start writing.** Not available. Every source file must carry an
SPDX identifier and CI enforces it, so the first line of `locusd` requires this
answer. That constraint is a feature: it stops the licence being something nobody
chose.
