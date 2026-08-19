<!-- SPDX-License-Identifier: Apache-2.0 -->
# Contributing to GitLocus

## The short version

Open a pull request. Sign your commits off with `git commit -s`. Run the four
checks below before you push. Say what you actually verified.

```bash
cargo build --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

## AI-assisted contributions

**Allowed, explicitly and without qualification.** This project would be
incoherent otherwise — it exists because agent-produced work is worth evaluating
properly.

What is required is not a particular authorship. It is this:

> **A named human is answerable for every contribution.**

Your `Signed-off-by` line is that statement. It means you have read the change,
you understand what it does, and you can answer questions about it. It does not
matter whether you typed it, an agent typed it, or you and an agent went back and
forth for an hour.

What this project will not do is try to detect which of those happened. That
detection does not work, and pretending otherwise would make the whole project
dishonest. See [ADR 0002](docs/adr/0002-no-ai-authorship-detection.md).

### What gets a contribution closed

Not "it was AI-assisted". These:

- **Claims you did not verify.** Saying the tests pass when you did not run them.
  This is the one that costs maintainers the most, because the whole point of a
  claim is to save someone the work of checking, and a false one does the
  opposite.
- **No responsible human.** A pull request whose author cannot answer questions
  about it.
- **Volume over substance.** A stream of unrelated changes opened faster than any
  human could have read the result.

If you are unsure whether something is wanted, open a discussion first. That is
free and always faster than a rejected pull request.

## Developer Certificate of Origin

This project uses the [DCO](https://developercertificate.org/), not a CLA. There
is nothing to sign and no account to create — just add `-s` to your commits:

```bash
git commit -s -m "your message"
```

This appends `Signed-off-by: Your Name <your@email>`, which certifies you have the
right to submit the work under Apache-2.0. CI checks for it.

We chose DCO over a CLA deliberately: a CLA asks a stranger for a legal
undertaking before their first contribution, and a machine can verify a DCO but
not a CLA's intent.

## How the gate works

Every pull request is evaluated against [`.gitlocus/policy.yml`](.gitlocus/policy.yml)
by this project's own tool. You can run it yourself; it is the same code CI runs,
so the answers cannot differ.

Checks fall into three classes, and the difference matters:

- **Deterministic** — build, tests, clippy, fmt, schema validation. These block.
  Anyone can re-run them and get the same answer.
- **Assessed** — model or heuristic findings. These **block nothing**, ever. None
  are produced yet; the verdict already renders them and a reviewer arrives in
  Stage 1.
- **Attested** — a human approving. Required by a branch ruleset rather than by a
  workflow, because a workflow could be edited in the same pull request it
  governs.

Some paths need more:

| touching | also requires |
|---|---|
| `.github/workflows/**`, `.gitlocus/**` | maintainer standing, plus a workflow audit |
| `spec/**` | contributor standing, plus schema validation and matching conformance tests |

CI configuration is gated hardest because it decides what every other check is
worth.

## Changing the spec

`spec/` is a contract other implementations may code against. A change there needs:

1. the normative text updated,
2. the JSON Schemas updated,
3. the conformance suite in `crates/gitlocus-core/tests/conformance.rs` updated to match.

Prose and code drifting apart between releases is the characteristic failure of
specification repositories, so the policy requires all three.

## Style

- `// SPDX-License-Identifier: Apache-2.0` at the top of every source file.
- Doc comments on public items — `missing_docs` warns and CI denies warnings.
- Comments explain **why**. A comment restating the code should be deleted.
- Name tests as the claim they make.

## Reporting security issues

Do not open a public issue. See [SECURITY.md](SECURITY.md).

## Finding something to work on

The [issue list](https://github.com/hey-vera/gitlocus/issues) is the work, and
[milestones](https://github.com/hey-vera/gitlocus/milestones) are the stages.
Issues labelled `open-question` are genuinely undecided — argue with them before
building anything, since a good argument is worth more there than a pull
request.

Decisions already taken live in [`docs/adr/`](docs/adr/). If you think one is
wrong, say which part of its context has changed rather than reopening the
conclusion.

## Code of conduct

[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) applies to everyone here.
