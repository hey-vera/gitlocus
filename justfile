# SPDX-License-Identifier: Apache-2.0
#
# Every check this repository runs, defined once.
#
# The recipes below are named for the required status checks in the `main`
# ruleset, and the workflows in .github/workflows/ invoke them rather than
# restating them. That is the point: `AGENTS.md` used to claim the four commands
# it listed were "the same commands CI runs", and they were not — CI passed
# `--locked` on all of them and ran nine more checks besides, so a change could
# be green locally and red on eleven required checks. A claim about what CI runs
# can only be kept true by being the thing CI runs.
#
# `.github/required-checks.txt` maps each required check to its recipe, and
# `crates/repo-conformance` fails if a check has no job, or a job has no recipe.
#
# Local prerequisites, and how to get them:
#
#   cargo deny     cargo install --locked cargo-deny
#   cargo mutants  cargo install --locked cargo-mutants
#   zizmor         uvx zizmor  (or: pipx install zizmor)
#
# On Windows under MSYS or Git Bash, the default host toolchain resolves to an
# MSVC target where MSYS `link` shadows MSVC's linker and every build fails at
# the link step. Export the toolchain that works on your machine:
#
#   export LOCUS_CARGO="cargo +stable-x86_64-pc-windows-gnullvm"

set shell := ["bash", "-euo", "pipefail", "-c"]

# `-D warnings` is the workspace's real lint posture, so it belongs here rather
# than in workflow env where a local run never sees it. `mutants` clears it
# deliberately; see that recipe.
export RUSTFLAGS := env("RUSTFLAGS", "-D warnings")

cargo := env("LOCUS_CARGO", "cargo")

# `python3` is not a command on Windows, where the launcher is `py`. The
# workflows run on Linux and pass `python3` through this variable unchanged.
python := env("LOCUS_PYTHON", if os_family() == "windows" { "py" } else { "python3" })

_default:
    @just --list --unsorted

# The four fast checks, for the edit-compile loop. Not a substitute for `just ci`.
check: build tests lint fmt

# Everything runnable off a GitHub runner. `mutants` and `gate` need a base
# revision and are excluded; `analyze (rust)` and `analyze (actions)` are CodeQL
# and run only on GitHub's analysis infrastructure.

# Everything runnable off a GitHub runner.
ci: build tests lint fmt msrv platforms schema-validation workflow-audit cargo-deny licence-headers

# --- required checks, one recipe per status-check context ---------------------

# Compile every target, with the lockfile as committed.
build:
    {{ cargo }} build --all-targets --locked

# The test suite, the doctests, and the evidence-collector tests.
tests:
    {{ cargo }} test --all-targets --locked
    {{ cargo }} test --doc --locked
    {{ python }} scripts/test_collect_evidence.py

# Clippy over every target, warnings denied.
lint:
    {{ cargo }} clippy --all-targets --locked -- -D warnings

# Formatting, checked not applied.
fmt:
    {{ cargo }} fmt --all --check

# `rust-version` in the workspace manifest is a promise to anyone whose
# toolchain is older than ours. It was declared and never compiled against, which
# made it an assertion rather than a fact.
#
# The version is read out of the manifest rather than written here twice. A
# second copy would drift, and silently in the direction that matters: the
# manifest would claim support nothing ever checked.

# Compile the workspace on the declared MSRV.
msrv:
    #!/usr/bin/env bash
    set -euo pipefail
    msrv=$(grep -m1 '^rust-version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
    if [ -z "$msrv" ]; then
      echo "::error::no rust-version in the workspace manifest"
      exit 1
    fi
    echo "manifest declares rust-version = $msrv"
    rustup toolchain install "$msrv" --profile minimal --no-self-update
    cargo "+$msrv" build --locked --all-targets

# On a runner this is one leg of a three-target matrix that CI collapses into a
# single check name. Locally it is the native target, which is the leg you can
# actually run.

# The test suite on this host's native target.
platforms:
    {{ cargo }} test --locked --all-targets

# The JSON Schemas are valid, and this repository's own policy parses.
schema-validation:
    {{ python }} scripts/check_schemas.py
    {{ cargo }} run --locked -p gitlocus-cli -- policy check --policy .gitlocus/policy.yml

# Do the tests actually constrain the implementation, or do they merely run it?
# Coverage cannot see an assertion that was loosened rather than deleted, because
# loosening one executes the same lines. A surviving mutant can, because it means
# nothing asserted the behaviour the mutant changed. ADR 0015.
#
# Scoped to the diff, so cost tracks the size of the change rather than the size
# of the codebase.

# Mutation-test the lines this branch changed.
mutants base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    # `-D warnings` has to come off. A mutant often leaves an unused binding
    # behind, and under -D warnings that fails to compile — cargo-mutants scores
    # it "unviable" rather than running it, so the strict setting silently
    # shrinks the number of mutants actually tested. Warnings are still denied by
    # `just lint`, which is where they belong.
    export RUSTFLAGS=""
    # Two dots, not three, to match how `locus contribution` computes changed
    # paths: the comparison is between the revisions as they are.
    git diff "{{ base }}" HEAD > pr.diff
    trap 'rm -f pr.diff' EXIT
    if [ ! -s pr.diff ]; then
      echo "no changes to mutate"
      exit 0
    fi
    # A timed-out mutant is timing-dependent and therefore cannot be
    # deterministic evidence. cargo-mutants exits non-zero for both surviving
    # mutants and timeouts, and this fails on either — stricter than
    # inconclusive, so nothing is laundered into a pass. --no-shuffle keeps the
    # ordering, and therefore the log, reproducible.
    #
    # The whole workspace. This was scoped to gitlocus-core while the CLI had no
    # behavioural tests and 24 surviving mutants; #34 closed that, and both
    # crates now report zero survivors, so there is nothing left to exclude.
    {{ cargo }} mutants --in-diff pr.diff --no-shuffle --colors=never

# What the *base* revision's gate demands of the working tree. Built from the
# base so the contribution cannot compile the evaluator that judges it — ADR
# 0014. Evidence comes from check runs and exists only on GitHub, so this shows
# the requirements rather than a full verdict.

# What the base revision's policy demands of this branch.
gate base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    base_sha=$(git rev-parse "{{ base }}")
    worktree=$(mktemp -d)
    trap 'git worktree remove --force "$worktree" 2>/dev/null || true' EXIT
    git worktree add --detach "$worktree" "$base_sha" >/dev/null
    {{ cargo }} build --release --locked -p gitlocus-cli --manifest-path "$worktree/Cargo.toml"
    echo "gate built from $base_sha: $(./target/release/locus --version)"
    git show "$base_sha:.gitlocus/policy.yml" > "$worktree/base-policy.yml"
    ./target/release/locus policy check --policy "$worktree/base-policy.yml"

# zizmor audits workflows for injection, over-broad permissions, unpinned
# actions and credential persistence. CI runs it through the action so findings
# upload as SARIF; here it runs directly.

# Audit the workflows: zizmor, plus the structural rules below.
workflow-audit: structural-rules
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v zizmor >/dev/null 2>&1; then
      echo "::error::zizmor is not installed. Try: uvx zizmor --persona pedantic --min-severity low .github/workflows/"
      exit 1
    fi
    zizmor --persona pedantic --min-severity low .github/workflows/

# The two rules this project treats as absolute, which zizmor does not assert.
structural-rules:
    #!/usr/bin/env bash
    set -euo pipefail
    fail=0

    # The pull-request-target trigger runs with a writable token in the context
    # of the base repository. In a repository whose whole purpose is evaluating
    # untrusted contributions, it is banned outright.
    #
    # Matched as a YAML key at the start of a line, so that this check does not
    # trip over its own error message.
    if grep -rnE '^[[:space:]]*(-[[:space:]]*)?pull_request_target[[:space:]]*:' .github/workflows/; then
      echo "::error::the pull-request-target trigger is banned in this repository"
      fail=1
    fi

    # Every action reference must be a full 40-character commit SHA. A tag is
    # mutable and a mutable dependency in CI is a supply-chain hole.
    while IFS= read -r line; do
      ref="${line##*@}"
      if ! printf '%s' "$ref" | grep -Eq '^[0-9a-f]{40}$'; then
        echo "::error::unpinned action reference: $line"
        fail=1
      fi
    done < <(grep -rhoE '^\s*-?\s*uses:\s*\S+' .github/workflows/ | sed -E 's/.*uses:\s*//')

    [ "$fail" -eq 0 ] && echo "workflow structure ok"
    exit "$fail"

# Advisories, licences and bans.
cargo-deny:
    {{ cargo }} deny --all-features check

# The licence map is executable rather than documentary. Apache-2.0 everywhere
# except the server, which is AGPL-3.0 — ADR 0016.

# Every source file carries the SPDX identifier its directory requires.
licence-headers:
    #!/usr/bin/env bash
    set -euo pipefail
    fail=0
    while IFS= read -r f; do
      case "$f" in
        crates/locusd/*) want="AGPL-3.0-only" ;;
        *)               want="Apache-2.0" ;;
      esac
      if ! head -5 "$f" | grep -q "SPDX-License-Identifier: $want"; then
        echo "::error file=$f::expected SPDX-License-Identifier: $want"
        fail=1
      fi
    done < <(find crates scripts -type f \( -name '*.rs' -o -name '*.py' \))
    [ "$fail" -eq 0 ] && echo "every source file carries the identifier its directory requires"
    exit "$fail"

# The sign-off is this project's accountability mechanism: it is the statement
# that a named human will answer for the change.

# Every commit on this branch is signed off.
dco base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    fail=0
    while IFS= read -r sha; do
      [ -z "$sha" ] && continue
      if ! git show -s --format=%B "$sha" | grep -qiE '^Signed-off-by: .+ <.+@.+>'; then
        subject=$(git show -s --format=%s "$sha")
        echo "::error::missing Signed-off-by on $sha: $subject"
        fail=1
      fi
    done < <(git rev-list "{{ base }}"..HEAD)
    if [ "$fail" -ne 0 ]; then
      echo "Fix with: git commit --amend -s   (or: git rebase --signoff {{ base }})"
      exit 1
    fi
    echo "all commits signed off"

# --- orientation --------------------------------------------------------------

# Where the project is, read from GitHub rather than from a document that has to
# be remembered. Nothing here is committed, so nothing here can go stale.

# Where the project is right now, read live from GitHub.
brief:
    #!/usr/bin/env bash
    set -euo pipefail
    repo=hey-vera/gitlocus

    echo "== milestones =="
    gh api "repos/$repo/milestones?state=open" \
      --jq '.[] | "\(.title)  open:\(.open_issues) closed:\(.closed_issues)"'

    echo
    echo "== open issues by milestone =="
    gh issue list --repo "$repo" --state open --limit 100 \
      --json number,title,milestone \
      --jq 'group_by(.milestone.title // "(unmilestoned)")[]
            | "\n\(.[0].milestone.title // "(unmilestoned)")"
            , (.[] | "  #\(.number) \(.title)")'

    echo
    echo "== latest release =="
    gh release view --repo "$repo" --json tagName,publishedAt,isLatest \
      --jq '"\(.tagName)  \(.publishedAt)  latest:\(.isLatest)"'

    echo
    echo "== last five merges =="
    git log --oneline -5 origin/main

    echo
    echo "== live service =="
    curl -fsS https://locus.heyvera.org/healthz || echo "unreachable"
    echo

    echo
    echo "== required checks: ruleset vs .github/required-checks.txt =="
    # The conformance suite reads the checked-in file so it can run offline. This
    # is the other half: the file is only worth trusting if it still matches the
    # ruleset that actually blocks merges.
    gh api "repos/$repo/rulesets" --jq '.[] | select(.name=="main") | .id' \
      | while read -r id; do
          gh api "repos/$repo/rulesets/$id" \
            --jq '.rules[] | select(.type=="required_status_checks")
                  | .parameters.required_status_checks[].context' \
            | sort > /tmp/locus-live-checks.txt
        done
    sed -E 's/[[:space:]]*=.*//' .github/required-checks.txt \
      | grep -v '^\s*#' | grep -v '^\s*$' | sort > /tmp/locus-file-checks.txt
    if diff -u /tmp/locus-file-checks.txt /tmp/locus-live-checks.txt; then
      echo "required-checks.txt matches the ruleset"
    else
      echo "::error::required-checks.txt has drifted from the main ruleset"
      exit 1
    fi
