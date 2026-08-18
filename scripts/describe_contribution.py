#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build the Contribution document for the pull request under evaluation.

Reads its inputs from the environment rather than from arguments, so that a
branch name or a login - both attacker-controlled - never reaches a shell.
"""

import json
import os
import sys

# Until the trust graph exists, the forge's own view of the author is the only
# tier signal available. It is a weak one, and the specification says so.
TIERS = {
    "OWNER": "maintainer",
    "MEMBER": "maintainer",
    "COLLABORATOR": "maintainer",
    "CONTRIBUTOR": "contributor",
}


def main():
    try:
        repo = os.environ["REPO"]
        base = os.environ["BASE_SHA"]
        head = os.environ["HEAD_SHA"]
        author = os.environ["AUTHOR"]
        association = os.environ["ASSOCIATION"]
    except KeyError as missing:
        print(f"missing required environment variable: {missing}", file=sys.stderr)
        return 1

    tier = TIERS.get(association, "unknown")

    with open("changed.txt", encoding="utf-8") as handle:
        paths = [line.strip() for line in handle if line.strip()]

    contribution = {
        "repository": f"github.com/{repo}",
        "base_digest": base,
        "head_digest": head,
        "actor": {
            "id": author,
            "kind": "human",
            "tier": tier,
            "key_binding": "https://token.actions.githubusercontent.com",
        },
        "changed_paths": paths,
    }

    with open("contribution.json", "w", encoding="utf-8") as handle:
        json.dump(contribution, handle, indent=2)

    print(f"{len(paths)} changed path(s); author {author} at tier {tier}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
