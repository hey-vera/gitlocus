<!-- SPDX-License-Identifier: Apache-2.0 -->
# LEARNINGS.md

What this project got wrong, and what now stops it happening again.

Two append-only lists. Both carry a **guard** — the thing that would catch a
recurrence — because a learning without one is a note, and notes decay into
folklore that a new contributor has no reason to read. A guard is one of:

- a **test name**, which must exist in this workspace;
- a **path**, which must exist;
- an **issue**, for a guard that is wanted and not built;
- `none — <reason>`, for a one-time mistake nothing can usefully prevent.

`every_learning_has_a_guard` in [`crates/repo-conformance`](crates/repo-conformance)
checks that column. It can only check that the claim to have a guard is not
empty; whether the guard is any good is still a judgement, and stating one that
does not hold would be the same failure as the rows below.

These two lists lived in [AGENTS.md](AGENTS.md) until they grew long enough to
dilute the seven invariants, which are the part of that document nobody may
skim. They are moved rather than shortened: the record is the culture.

## Claims shipped stronger than the implementation

The standard is that every claim in this repository is backed by something that
runs. It is the product thesis applied to the project's own documentation, and
it is the thing most likely to slip. It has slipped, every time in the same
direction:

| claimed | actual | guard |
|---|---|---|
| "GitLocus reads `VOUCHED.td`" | no reader existed | `a_vouch_raises_an_unknown_actor_but_does_not_demote_a_higher_one` |
| "the release is immutable" | a repository setting, never enabled | `just brief` — enabled 2026-08-19; `brief` fails if it is turned off. Releases published before that stay mutable, so `brief` still reports `immutable:false` for v0.0.3. True from the next tag, not retroactively. |
| the gate reported a verdict on "the evidence" | it could only see its own workflow | `scripts/collect_evidence.py` — evidence comes from the check-runs API, not from a job's `needs` |
| approvals were counted | from a self-asserted string | `a_forged_signer_in_input_json_cannot_satisfy_a_signed_requirement` |
| README announced v0.0.1 | v0.0.2 had shipped | `every_version_string_matches_the_workspace_version` |
| a `Code of Conduct` reporting address | `gitlocus.dev` does not resolve; it would have bounced | none — one-time; a live-domain check would make CI depend on DNS |
| `skipped` counted as a deterministic pass | invariant 3 violated in the product, not the config | `scripts/test_collect_evidence.py` |
| crates named `locus-core` / `locus-cli` | both already taken on crates.io; unpublishable | none — one-time, and the names are now taken by us |
| a step named "Publish immutable release" | see row two | `just brief` |
| the gate evaluated "the policy" | it evaluated the one the pull request shipped, so a change deleting every rule came back `satisfied` | `clause_6_a_contribution_cannot_weaken_the_policy_that_governs_it` |
| conformance clause 6 was claimed | it was the only clause with no test, which is why the row above survived | `crates/gitlocus-core/tests/conformance.rs` |
| `locus --version` on the v0.0.2 release | reported `0.0.1`; the workspace version was never bumped | `every_version_string_matches_the_workspace_version` |
| the ruleset "restricts pushes to those paths to code owners" | it carries no path restriction at all | none — the claim was withdrawn; `.gitlocus/policy.yml` enforces `min_tier: maintainer` on those paths instead, which is the product's own mechanism |
| the approval requirement gated merges | every merge to `main` logged `result=bypass`, so no rule in the ruleset had ever been evaluated | `just brief` — fails if the `main` ruleset gains a bypass actor |
| the four documented commands were "the same commands CI runs" | CI passed `--locked` on all four and ran nine more checks besides, so a change could be green here and red on eleven required checks | `every_required_check_has_a_job_and_a_recipe` |
| "Dependabot security updates: enabled" | reported by the repository API while the dependency graph is **not** enabled, so `dependency-graph/compare` returns 403 and dependency review cannot run at all | https://github.com/hey-vera/gitlocus/issues/67 |
| `cargo test --doc` kept "the model's documentation examples honest" | every fenced block in a doc comment is `yaml` or `text`, which rustdoc does not compile; the step reported `running 0 tests` | `every_yaml_example_in_the_documentation_is_a_valid_policy` |

Run it, read the output, quote it. Under-claiming costs nothing; over-claiming
costs the benefit of the doubt on everything else you say.

**The structural reason this keeps happening:** claims live in prose, and prose
has no CI. The durable fix is not to be more careful — it is to convert a claim
into something that runs. That is the product thesis applied to the repository
that ships it, and where a claim cannot be made executable, it should be written
as the weaker thing that is true.

The guard column is that rule made unavoidable. Adding a row without asking what
would catch it is how this becomes a list of regrets instead of a list of checks.

## Traps worth not rediscovering

Each of these cost real time at least once. The guard is what makes the next
person's encounter with it cheap, or says plainly that nothing does.

### Local Rust needs an explicit toolchain

Use `cargo +stable-x86_64-pc-windows-gnullvm`. The default toolchain resolves to
an MSVC target where MSYS `link` shadows MSVC's linker, and every build fails at
the link step with an error that names neither.

**Guard:** `justfile` — export `LOCUS_CARGO` and every recipe uses it. The
justfile header carries the exact line.

### Windows CI runners default to PowerShell

Where `"$VAR"` silently expands to nothing. Set `shell: bash` as a job default,
not per step, so a step added later cannot reintroduce it.

**Guard:** none — zizmor does not check this and neither does anything else.
The `platform (x86_64-pc-windows-msvc)` leg catches the consequences, which is
weaker than catching the cause.

### `pull_request_target` is banned here

Several actions document that trigger as their normal usage. Use `pull_request`
and accept the reduced behaviour on fork contributions. In a repository whose
whole purpose is evaluating untrusted contributions, a writable token in the
base repository's context is not a trade worth making.

**Guard:** `just structural-rules`, a required check via `workflow-audit`.

### `gh attestation verify` already yields the right identity shape

Signer identities come out shaped exactly like a `signed_by` glob. Try that
before building a signing path on cosign.

**Guard:** https://github.com/hey-vera/gitlocus/issues/8

### A permissive `signed_by` glob is close to no constraint

Anyone can run a workflow in their own fork and get a valid identity from the
same issuer. Pin the workflow path.

**Guard:** https://github.com/hey-vera/gitlocus/issues/8

### The pull request body must end with a DCO trailer

This repository sets `web_commit_signoff_required` and takes the squash message
from `PR_BODY`, so a body without `Signed-off-by:` produces a squash commit that
the setting refuses. The failure surfaces as "the base branch policy prohibits
the merge", which points at the ruleset and is nothing to do with it.

**Guard:** none — the `dco` check covers commits, not the pull request body, and
nothing can check the body before the merge is attempted.

### Commits must be signed, and the signing key is separate from the auth key

`required_signatures` is in the `main` ruleset. Sign with
`~/.ssh/id_ed25519_signing`, which has no passphrase so unattended commits work;
`~/.ssh/id_ed25519` is passphrase-protected and cannot sign in a script.

**Guard:** the `main` ruleset, which `just brief` asserts still carries
`required_signatures` with no bypass actor.

### `just msrv` cannot run on a host the MSRV predates

The recipe installs the MSRV toolchain for the same host triple `LOCUS_CARGO`
names, because installing it for the default host would walk straight into the
linker trap above. Rust 1.90 shipped no `x86_64-pc-windows-gnullvm` host
toolchain, so on Windows under MSYS this check cannot run at all.

It fails rather than skips, and says where it does run — the `msrv` job on
`ubuntu-latest`, which is a required status check. `just ci` runs it last, so a
host that cannot run it still gets every other check.

**Guard:** `justfile` — the recipe prints the constraint and the exact list of
recipes to run instead.

### Nothing built the documentation until 2026-08-19

`cargo doc` ran nowhere. `evidence.rs` and `policy.rs` are dense with
`[`crate::thing`]` links and a broken one failed in no check — the first person
to notice would have been a reader on docs.rs.

**Guard:** `just doc`, which denies `rustdoc::broken_intra_doc_links` and is run
by `just lint`, a required check.

### The release workflow is the highest-privilege thing here

`publish` holds `contents: write`, `id-token: write` and `attestations: write`,
and it is where an unpinned action or a careless `run:` costs the most. Anything
added to it — an SBOM generator, a signer — is code with a Sigstore certificate
in reach.

**Guard:** `just structural-rules` requires every action reference to be a full
40-character SHA, and the `release` environment has a required reviewer, so a
tag push cannot reach any of it unattended.

### Do not stack pull requests here

`delete_branch_on_merge` is on, and when a base branch is deleted GitHub marks
every pull request targeting it as *merged* — closed, unreopenable, and with none
of its content in `main`. Two slices were lost to this. Target `main` and land
one at a time.

**Guard:** none — GitHub offers no setting for this. It is a working rule, and
the reason this document says it twice.

### Auto-merge is armed on every pull request

One merges the moment its checks go green. Push every commit you intend to
include *before* that happens; a follow-up pushed to a branch that has already
merged is stranded and needs its own pull request. This has happened twice.

**Guard:** `.github/workflows/automerge.yml` — the behaviour is deliberate
and checked in, so at least it is discoverable rather than surprising.

### On `pull_request`, `actions/checkout` gives you the merge ref

Anything read out of the working tree is the pull request's version of it. That
is correct for the code under test and wrong for anything that decides whether
the code may merge — see
[ADR 0013](docs/adr/0013-a-contribution-is-governed-by-base-and-head.md).

It bites in smaller ways too: `base..HEAD` includes GitHub's generated merge
commit, which carries no sign-off, so a `dco` check written that way fails on a
branch where every commit is signed off. That happened while moving the check
into a recipe, and the check caught it on the first run.

**Guard:** `docs/adr/0014-the-gate-is-built-from-the-base-revision.md` for
the gate; the `dco` job passes both revisions explicitly.
