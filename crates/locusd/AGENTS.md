<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# AGENTS.md — crates/locusd

The repository-wide instructions in [`../../AGENTS.md`](../../AGENTS.md) apply
here too. This file exists for the one thing that is different, because the
[AGENTS.md convention](https://agents.md) has an agent read the nearest file in
the directory tree — so this arrives without anyone remembering to mention it.

**This crate is AGPL-3.0-only. The rest of the workspace is Apache-2.0.**

Every file here starts with `// SPDX-License-Identifier: AGPL-3.0-only`, not the
Apache identifier used everywhere else. `just licence-headers` enforces the split
by directory and is a required check, so getting it wrong fails rather than
merges. [ADR 0016](../../docs/adr/0016-locusd-lives-here-under-agpl.md) is why
the licence is split by directory instead of the code by repository.

Two consequences that are easy to get wrong:

- **Do not move code from here into `gitlocus-core` or `gitlocus-cli`** without
  deciding the licence question first. Copying an AGPL file into an Apache crate
  is a relicensing decision, and it is not one to take by accident while
  refactoring.
- **`gitlocus-core` may not depend on this crate.** The dependency runs one way,
  and the reverse would pull AGPL obligations into the crate other people are
  meant to build on.

The service also has no network access beyond the socket it is handed, and
`evaluate` stays a pure function of its inputs — the same purity invariant the
core has, for the same reason.
