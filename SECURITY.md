<!-- SPDX-License-Identifier: Apache-2.0 -->
# Security Policy

## Reporting a vulnerability

Use GitHub's [private vulnerability reporting](https://github.com/hey-vera/gitlocus/security/advisories/new).
Do not open a public issue.

You will get an acknowledgement within 7 days and an assessment within 30.

## There is no bug bounty, and there will not be one

In January 2026 curl ended its bug bounty after six years, 87 valid findings and
over $100,000 paid out. The valid-report rate had fallen from above 15% to below
5% while volume rose. The money had stopped buying signal and started buying
noise.

We are stating this up front rather than discovering it later. Reports are welcome
and will be read. They are not paid.

## What a report must contain

A report we cannot act on costs us the same attention as one we can, which is the
asymmetry this entire project is about. Please include:

1. **The version or commit** you tested.
2. **Reproduction steps** that we can run. A proof of concept, a failing test, or
   an exact command sequence.
3. **What you observed**, distinguished from what you inferred.
4. **The impact**, concretely: what can an attacker do that they should not be
   able to do?

If a tool or model produced the report, that is fine — say so, and say what you
verified yourself. A report with "I confirmed steps 1-3 by hand, step 4 is model
output I could not reproduce" is more useful than one presented as uniformly
confirmed, and it will be treated better.

## What will be closed without detailed response

- Reports with no reproduction steps.
- Output pasted from a scanner with no analysis of whether it applies here.
- Claims about code paths that do not exist in this repository.
- Theoretical findings with no described impact.

This is not hostility toward automated tooling. It is the same principle the
project encodes: a claim that cannot be checked cheaply is worth less than one
that can, and the burden of making it checkable belongs with whoever is making it.

## Scope

In scope: this repository — the `gitlocus-core` and `gitlocus-cli` crates, the schemas
in `spec/`, and the workflows in `.github/workflows/`.

Of particular interest, because they are where a real bug would hurt most:

- Any way to make **assessed** evidence satisfy a blocking requirement.
- Any way to make evidence bound to one revision count for another.
- Any way to make verdict evaluation non-deterministic.
- Any way to have a policy evaluated from a revision other than the one under
  evaluation.
- Workflow injection or privilege escalation in CI.

Out of scope: GitHub's own infrastructure, and denial of service through
volume alone.

## Supported versions

Pre-1.0. Only the latest commit on `main` is supported. There are no backports.
