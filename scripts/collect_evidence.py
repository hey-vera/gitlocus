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

# The gate cannot wait on itself. Auto-merge arming is not a claim about the
# code. Scorecard and CodeQL report repository posture rather than a verdict on
# this revision, and the policy does not require them.
EXCLUDED = re.compile(r"^(gate|arm|Scorecard|analyze)")

# Only two conclusions mean the check actually ran and was satisfied.
PASSING = {"success", "neutral"}

# Only one means it ran and was not.
FAILING = {"failure"}

# Everything else — skipped, cancelled, stale, timed_out, action_required —
# means no answer was reached. That is `inconclusive`, which the model treats as
# unmet, and it must never be read as a pass.
#
# `skipped` was previously in PASSING. It was harmless only because nothing in
# this repository is ever skipped, and it would have failed silently for the
# first adopter with a conditional job: a check that did not run would have
# satisfied the requirement it was meant to prove. That is invariant 3 in
# AGENTS.md, violated in the product rather than in the configuration.


def relevant(runs):
    seen = set()
    for run in runs:
        name = run["name"]
        if EXCLUDED.match(name) or name in seen:
            continue
        seen.add(name)
        yield run


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
        pending = sorted(r["name"] for r in relevant(runs) if r["status"] != "completed")
        if pending:
            print("pending: " + ", ".join(pending))
            return 1
        return 0

    head = os.environ["HEAD_SHA"]
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    evidence = []
    for run in relevant(runs):
        conclusion = run.get("conclusion") or ""
        if run["status"] != "completed":
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
