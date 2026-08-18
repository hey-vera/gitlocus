#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Every published schema must be valid JSON and free of remote references.

A remote $ref would make every consumer's validation depend on a live domain,
which is a poor property for a specification: the schema would stop working the
day the domain lapses, and it would leak who is validating what.
"""

import json
import pathlib
import sys

SCHEMA_DIR = pathlib.Path("spec/schemas")


def refs_in(node, found):
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str):
            found.append(ref)
        for value in node.values():
            refs_in(value, found)
    elif isinstance(node, list):
        for value in node:
            refs_in(value, found)
    return found


def main():
    schemas = sorted(SCHEMA_DIR.glob("*.json"))
    if not schemas:
        print(f"no schemas found under {SCHEMA_DIR}", file=sys.stderr)
        return 1

    failed = False
    for path in schemas:
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            print(f"{path}: not valid JSON: {exc}")
            failed = True
            continue

        remote = [r for r in refs_in(doc, []) if not r.startswith("#")]
        if remote:
            print(f"{path}: remote $ref not allowed: {remote}")
            failed = True
        else:
            print(f"{path}: ok")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
