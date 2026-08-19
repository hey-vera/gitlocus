#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Tests for the evidence collector.

The collector decides what the gate is allowed to believe, so its edge cases are
security-relevant rather than cosmetic. Two of the cases below are bugs this
project actually shipped: `skipped` counted as a pass, and a check reporting a
conclusion while still marked in progress counted as inconclusive.

Driven as a subprocess rather than imported, because the exclusion pattern is
read from the environment at import time and the environment is part of what is
being tested. No test framework: plain asserts keep this runnable anywhere
python3 is, which is the same bar the other scripts here meet.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

COLLECT = Path(__file__).resolve().parent / "collect_evidence.py"


def run(checks, *args, exclude=None):
    """Run the collector over `checks` and return (exit code, stdout, evidence)."""
    with tempfile.TemporaryDirectory() as work:
        Path(work, "checks.json").write_text(json.dumps(checks), encoding="utf-8")
        env = dict(os.environ, HEAD_SHA="bbbb2222")
        if exclude is not None:
            env["GITLOCUS_EXCLUDE"] = exclude
        else:
            env.pop("GITLOCUS_EXCLUDE", None)
        proc = subprocess.run(
            [sys.executable, str(COLLECT), *args],
            cwd=work,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        written = Path(work, "evidence.json")
        evidence = json.loads(written.read_text(encoding="utf-8")) if written.exists() else None
        return proc.returncode, proc.stdout, evidence


def check(name, status, conclusion):
    return {"name": name, "status": status, "conclusion": conclusion, "url": None}


def outcome_for(evidence, kind):
    return next(e["outcome"] for e in evidence if e["kind"] == kind)


def a_conclusion_is_authoritative_even_while_the_status_lags():
    # The bug this test exists for, observed on a real pull request: GitHub
    # reported `fmt` as status=in_progress and conclusion=success at the same
    # time, and held it for minutes. Reading status first recorded a check that
    # had passed as inconclusive, which the model correctly treats as unmet — so
    # a contribution was blocked by a check that agreed with it.
    _, _, evidence = run([check("fmt", "in_progress", "success")])
    assert outcome_for(evidence, "fmt") == "pass", evidence

    code, out, _ = run([check("fmt", "in_progress", "success")], "--pending-only")
    assert code == 0, f"a concluded check must not read as pending: {out}"


def a_check_with_no_conclusion_is_pending_and_inconclusive():
    _, _, evidence = run([check("tests", "in_progress", None)])
    assert outcome_for(evidence, "tests") == "inconclusive", evidence

    code, out, _ = run([check("tests", "in_progress", None)], "--pending-only")
    assert code == 1 and "tests" in out, out


def a_check_that_did_not_run_is_never_a_pass():
    # Invariant 3. `skipped` was once in the passing set, which would have let a
    # conditional job satisfy the requirement it was meant to prove.
    for conclusion in ("skipped", "cancelled", "stale", "timed_out", "action_required"):
        _, _, evidence = run([check("tests", "completed", conclusion)])
        assert outcome_for(evidence, "tests") == "inconclusive", conclusion


def real_outcomes_map_to_pass_and_fail():
    for conclusion in ("success", "neutral"):
        _, _, evidence = run([check("tests", "completed", conclusion)])
        assert outcome_for(evidence, "tests") == "pass", conclusion

    _, _, evidence = run([check("tests", "completed", "failure")])
    assert outcome_for(evidence, "tests") == "fail", evidence


def the_exclusion_pattern_comes_from_the_environment():
    # An adopting repository names its jobs whatever it likes, and the one thing
    # that must always be excluded is the job running the collector: it is a
    # check run, it is in progress while it collects, and waiting for itself
    # would stall until the retry budget ran out.
    checks = [check("gitlocus", "in_progress", None), check("tests", "completed", "success")]

    code, out, _ = run(checks, "--pending-only")
    assert code == 1 and "gitlocus" in out, f"unexcluded, the job waits on itself: {out}"

    code, _, evidence = run(checks, "--pending-only", exclude="^gitlocus")
    assert code == 0
    _, _, evidence = run(checks, exclude="^gitlocus")
    assert [e["kind"] for e in evidence] == ["tests"], evidence


def every_record_is_deterministic_and_bound_to_the_revision():
    # Nothing in this collector may emit assessed evidence, and evidence for
    # another revision must never be produced in the first place.
    _, _, evidence = run([check("tests", "completed", "success")])
    assert evidence[0]["class"] == "deterministic", evidence
    assert evidence[0]["subject_digest"] == "bbbb2222", evidence


def duplicate_check_names_collapse_to_one_record():
    checks = [check("tests", "completed", "success"), check("tests", "completed", "failure")]
    _, _, evidence = run(checks)
    assert len(evidence) == 1, evidence


def main():
    tests = [
        a_conclusion_is_authoritative_even_while_the_status_lags,
        a_check_with_no_conclusion_is_pending_and_inconclusive,
        a_check_that_did_not_run_is_never_a_pass,
        real_outcomes_map_to_pass_and_fail,
        the_exclusion_pattern_comes_from_the_environment,
        every_record_is_deterministic_and_bound_to_the_revision,
        duplicate_check_names_collapse_to_one_record,
    ]
    failed = 0
    for test in tests:
        try:
            test()
            print(f"  ok    {test.__name__}")
        except AssertionError as error:
            failed += 1
            print(f"  FAIL  {test.__name__}: {error}")
    print(f"{len(tests) - failed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
