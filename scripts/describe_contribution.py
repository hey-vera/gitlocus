#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Build the Contribution document for the pull request under evaluation.

Reads its inputs from the environment rather than from arguments, so that a
branch name or a login - both attacker-controlled - never reaches a shell.

On deriving the trust tier
--------------------------
The obvious signal is `author_association` from the event payload, and it is
wrong in a way that is easy to miss: it reports only *publicly visible*
association. A maintainer whose organisation membership is private appears as
CONTRIBUTOR to the workflow while the REST API reports MEMBER. Trusting it means
maintainers are silently under-privileged, which surfaces as a gate that blocks
its own maintainers on paths only they are allowed to touch.

The repository collaborator permission is authoritative and does not depend on
profile visibility, so it is preferred when it can be read. It cannot always be:
a pull request from a fork carries a read-only token. The association is the
fallback, and which source was used is logged, because a tier derived from a
weaker signal should not look identical to one derived from a stronger one.

Either way this is a stopgap. Tier assignment is out of scope in v0 of the
specification and is currently the weakest link in the chain; see THREAT-MODEL.md.
"""

import json
import os
import sys

# Repository permission, when readable. Authoritative.
BY_PERMISSION = {
    "admin": "maintainer",
    "maintain": "maintainer",
    "write": "contributor",
    "read": "unknown",
    "none": "unknown",
}

# Public association, as a fallback. Understates private membership.
BY_ASSOCIATION = {
    "OWNER": "maintainer",
    "MEMBER": "maintainer",
    "COLLABORATOR": "maintainer",
    "CONTRIBUTOR": "contributor",
}


def derive_tier(permission, association):
    """Return (tier, source)."""
    if permission and permission in BY_PERMISSION:
        return BY_PERMISSION[permission], f"repository permission {permission!r}"
    return (
        BY_ASSOCIATION.get(association, "unknown"),
        f"public association {association!r} (permission unreadable)",
    )


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

    permission = os.environ.get("PERMISSION", "").strip()
    tier, source = derive_tier(permission, association)

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

    print(f"{len(paths)} changed path(s)")
    print(f"author {author} at tier {tier}, from {source}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
