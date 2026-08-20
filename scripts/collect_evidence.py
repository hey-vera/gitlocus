#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Turn the forge's check runs into Evidence records.

Evidence is collected from the check-run API rather than inferred from the
calling job's dependencies. The distinction matters: the ci-and-policy rule
requires a workflow-audit check that lives in a different workflow, and a gate
that could only observe its own workflow would report a verdict on part of the
evidence while presenting it as the whole.

Every record here is deterministic - a check run either completed or it did not,
and anyone can re-run it. Nothing in this file may emit assessed evidence.
"""

import argparse
import datetime
import json
import os
import re
import sys

# The gate cannot wait on itself: it is a check run too, it is still in progress
# while it collects, and waiting for it to complete would deadlock until the
# retry budget ran out. Auto-merge arming is not a claim about the code, and
# Scorecard and CodeQL report repository posture rather than a verdict on this
# revision.
#
# Read from the environment because an adopting repository names its jobs
# whatever it likes, and the one thing that must always be excluded — the job
# running this collector — is a name only the caller knows.
EXCLUDED = re.compile(os.environ.get("GITLOCUS_EXCLUDE") or r"^(gate|arm|Scorecard|analyze)")

# Only two conclusions mean the check actually ran and was satisfied.
PASSING = {"success", "neutral"}

# Only one means it ran and was not.
FAILING = {"failure"}

# Everything else — skipped, cancelled, stale, timed_out, action_required —
# means no answer was reached. That is `inconclusive`, which the model treats as
# unmet, and it must never be read as a pass.
#
# `skipped` was previously in PASSING. It was harmless only because nothing in
# this repository is ever skipped, so the fix went in unexercised.
#
# It has since been exercised. `1xmint/notelocus` gates a job behind a detect
# job, which is the ordinary way a repository avoids running a check it does not
# need, and on 2026-08-20 that produced a genuinely skipped check run for the
# first time. The gate reported:
#
#     inconclusive fixtures
#
# which is correct: skipped is not a pass, and inconclusive is not a pass either.
# Recorded here because "handled in code, never seen" and "seen, and it worked"
# are different claims and only one of them was true before that run.


def relevant(runs):
    seen = set()
    for run in runs:
        name = run["name"]
        if EXCLUDED.match(name) or name in seen:
            continue
        seen.add(name)
        yield run


def concluded(run):
    """Whether a check run has reached an answer.

    A conclusion is authoritative and terminal: the forge sets it only when the
    run has finished, and a re-run creates a new check run rather than editing
    this one. `status` is not authoritative on its own — GitHub has been observed
    reporting `status: in_progress` alongside `conclusion: success`, and holding
    that state for minutes.

    Reading `status` first meant a check that had *passed* was recorded as
    `inconclusive`, which the model correctly treats as unmet, so a green
    contribution was blocked by a check that agreed with it. Safe in direction
    and wrong in fact.
    """
    return run.get("status") == "completed" or bool(run.get("conclusion"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--pending-only",
        action="store_true",
        help="Exit 0 when every relevant check has completed, 1 otherwise.",
    )
    args = parser.parse_args()

    with open("checks.json", encoding="utf-8") as handle:
        runs = json.load(handle)

    if args.pending_only:
        pending = sorted(r["name"] for r in relevant(runs) if not concluded(r))
        if pending:
            print("pending: " + ", ".join(pending))
            return 1
        return 0

    head = os.environ["HEAD_SHA"]
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    evidence = []
    for run in relevant(runs):
        conclusion = run.get("conclusion") or ""
        if not concluded(run):
            outcome = "inconclusive"
        elif conclusion in PASSING:
            outcome = "pass"
        elif conclusion in FAILING:
            outcome = "fail"
        else:
            outcome = "inconclusive"

        record = {
            "kind": run["name"],
            "class": "deterministic",
            "outcome": outcome,
            "subject_digest": head,
            "produced_by": "github-actions",
            "produced_at": now,
        }
        if run.get("url"):
            record["source_uri"] = run["url"]
        evidence.append(record)

    with open("evidence.json", "w", encoding="utf-8") as handle:
        json.dump(evidence, handle, indent=2)

    for record in evidence:
        print(f"  {record['outcome']:<12} {record['kind']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
