<!-- SPDX-License-Identifier: Apache-2.0 -->
# gitlocus-core

The reference implementation of the GitLocus model: whether a change may enter a
trunk, decided reproducibly by anyone holding the same inputs.

```bash
cargo add gitlocus-core
```

Five types carry the whole model.

| type | what it is |
|---|---|
| `Actor` | Who produced a change — a human, an agent, or a pair — and what standing they hold. |
| `Contribution` | The proposed change, identified by content digests rather than by a forge's pull-request number. |
| `Evidence` | One claim about a contribution, in a class that decides what it may do. |
| `Policy` | What a repository demands, per path. |
| `Verdict` | The decision, and everything needed to recompute it. |

The normative definitions are in
[`spec/`](https://github.com/hey-vera/gitlocus/tree/main/spec), and this crate is
what the conformance suite tests against. If the two ever disagree, the spec is
right and this is a bug.

## The part that matters

Evidence carries a **class**, and the class is enforced in the type system rather
than by convention:

- `deterministic` — a check anyone can re-run and get the same answer from. Only
  this class can satisfy a requirement.
- `attested` — a claim by an identified party. Counts as an approval, never as a
  check.
- `assessed` — a model's judgement. Surfaced next to the verdict and **binds
  nothing**, at any score, behind any flag.

That third line is the point of the whole crate. A review model that has been
talked into approving something is a review model that approved something; the
class separation is what makes that survivable.

Two further guarantees the API is shaped around:

- **A verdict is a pure function of its inputs.** No clock, no network, no
  ambient state, and no dependence on the order of the evidence array — which is
  what makes a verdict content-addressable and therefore cacheable.
- **A signer is never read from input.** `Evidence::signer` is
  `skip_deserializing` on purpose: a signer is a conclusion the verifier reaches,
  not a claim the document gets to make.

## Using it

```rust
use gitlocus_core::Policy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = Policy::from_yaml(
        r#"
version: 0
rules:
  - name: baseline
    when:
      paths: ["**"]
    require:
      deterministic: [build, tests]
"#,
    )?
    .compile()?;

    // `policy.evaluate(&contribution, &evidence)` returns a Verdict.
    // `verdict.headline()` renders it; `verdict.exit_code()` is the decision.
    let _ = policy;
    Ok(())
}
```

`locus`, the command line tool in
[`gitlocus-cli`](https://crates.io/crates/gitlocus-cli), is this crate plus the
parts that touch a filesystem and a git repository — kept out of here so that
the model stays pure.

## Status

Pre-1.0 and the spec is a draft. Anything in `spec/` may break in a patch
release until v1; this crate follows it.

Licensed under Apache-2.0.
