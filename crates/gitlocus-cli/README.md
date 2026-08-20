<!-- SPDX-License-Identifier: Apache-2.0 -->
# gitlocus-cli

`locus` — decide whether a change may enter a trunk, and let anyone check the
answer.

```console
$ locus verify --policy .gitlocus/policy.yml --contribution c.json --evidence e.json
blocked: tests unmet
```

The exit code is the decision, so this drops into a shell script or a CI job
without parsing anything.

## Commands

| command | what it does |
|---|---|
| `locus verify` | Evaluate a contribution against a policy and print the verdict. |
| `locus policy check` | Parse and compile a policy, reporting what is wrong and where. |
| `locus contribution` | Describe the working tree's change as a `Contribution`. |
| `locus evidence emit` | Write an evidence record. Emitting never signs it. |
| `locus authorship declare` | Record a named human's authorship claim. |
| `locus vouch check` | Read a `VOUCHED.td` and report an identity's standing. |

Every command takes `-` for stdin and writes JSON, so they compose.

## Adopting it

On GitHub, the composite action is five lines and verifies the released binary's
attestation before running it:

```yaml
- uses: hey-vera/gitlocus@v0.0.3
```

Elsewhere, download a release binary and verify it offline against the bundle
published beside it:

```console
$ gh attestation verify locus-<target> --bundle locus.intoto.jsonl --repo hey-vera/gitlocus
```

## What it will not do

There is no flag that lets a model's judgement satisfy a requirement, and there
will not be one. `assessed` evidence is surfaced next to the verdict and binds
nothing. See
[`docs/adr/0005`](https://github.com/hey-vera/gitlocus/blob/main/docs/adr/0005-evidence-classes-survive-better-models.md),
which was written to answer the argument that models are reliable enough now, and
whose answer does not depend on model quality.

## Status

Pre-1.0. The model is [`gitlocus-core`](https://crates.io/crates/gitlocus-core)
and the normative definitions are in
[`spec/`](https://github.com/hey-vera/gitlocus/tree/main/spec).

Licensed under Apache-2.0.
