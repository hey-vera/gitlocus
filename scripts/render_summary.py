#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Render the gate's verdict as a GitHub step summary."""

import sys

EXPLANATION = {
    "satisfied": "Every requirement the policy asks for is present.",
    "needs_human": (
        "Machine checks are complete and agree. The outstanding approval is for "
        "your branch protection to enforce, not this workflow - a workflow can "
        "be edited in the same pull request it governs, so anything it enforced "
        "would be editable by whoever it was enforcing against."
    ),
    "blocked": "**Blocked.** See the unmet requirements above.",
}


def main():
    decision = sys.argv[1] if len(sys.argv) > 1 else "unknown"

    with open("verdict.txt", encoding="utf-8") as handle:
        verdict = handle.read().rstrip()

    fence = "```"
    print("## GitLocus gate")
    print()
    print(fence)
    print(verdict)
    print(fence)
    print()
    print(EXPLANATION.get(decision, f"Unrecognised decision: {decision}"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
