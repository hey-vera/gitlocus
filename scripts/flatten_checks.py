#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Normalise the check-run payload into the flat shape collect_evidence reads.

`gh api --paginate` emits one JSON array per page rather than a single array, so
the pages are concatenated here. Doing it in a file rather than in a shell
one-liner keeps the workflow readable and this logic testable.
"""

import json
import sys


def main():
    with open("raw.json", encoding="utf-8") as handle:
        text = handle.read().strip()

    if not text:
        json.dump([], open("checks.json", "w", encoding="utf-8"))
        return 0

    runs = []
    decoder = json.JSONDecoder()
    index = 0
    while index < len(text):
        # Skip whitespace between concatenated pages.
        while index < len(text) and text[index].isspace():
            index += 1
        if index >= len(text):
            break
        page, index = decoder.raw_decode(text, index)
        if isinstance(page, list):
            runs.extend(page)
        else:
            runs.append(page)

    flattened = [
        {
            "name": run.get("name", ""),
            "status": run.get("status", ""),
            "conclusion": run.get("conclusion"),
            "url": run.get("html_url"),
        }
        for run in runs
    ]

    with open("checks.json", "w", encoding="utf-8") as handle:
        json.dump(flattened, handle, indent=2)

    print(f"{len(flattened)} check run(s) observed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
