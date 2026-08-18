<!-- SPDX-License-Identifier: Apache-2.0 -->
## What this changes

<!-- One or two sentences. What is different after this merges? -->

## Why

<!-- The problem, not the solution. If it fixes an issue, link it. -->

## What you actually verified

<!--
This is the section that matters most here, and the one this project is
fundamentally about.

Say what you ran and what you observed. Under-claiming costs you nothing;
over-claiming costs you the benefit of the doubt on everything else you say.
"Tests pass" when you did not run them is worse than saying nothing, because a
claim exists to save a reviewer work and a false one does the opposite.
-->

- [ ] `cargo build --all-targets`
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --all --check`

Anything you could not verify, or verified only partially:

<!-- e.g. "did not test the Windows target; no machine to hand" -->

## Accountability

- [ ] Commits are signed off (`git commit -s`) - a named human answers for this change.

If an agent produced part or all of this work, that is fine and needs no
disclosure beyond your sign-off. See [CONTRIBUTING.md](../CONTRIBUTING.md).

## If this touches spec/

- [ ] Normative text updated
- [ ] JSON Schemas updated
- [ ] Conformance suite updated to match
