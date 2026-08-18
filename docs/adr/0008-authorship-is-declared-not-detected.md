<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0008 — Authorship is declared, not detected

- **Status:** accepted
- **Date:** 2026-08-18

## Context

Open source is being hollowed out, and the mechanism is not the one people
usually name.

The US Copyright Office's January 2025 report holds that AI output requires
"sufficient human control over the expressive elements" to be copyrightable, and
that **prompts alone — however detailed — do not supply it**. Code failing that
bar is uncopyrightable: effectively public domain.

**Public domain code inside a copyleft project is not bound by the copyleft.**

So copyleft does not die by being violated. It dies by **dilution**. Each merged
contribution that nobody can claim authorship of replaces enforceable code with
unenforceable code. Nothing visible breaks. At some point the licence is a
statement about nothing, and every contributor who gave their work under it has
been quietly written out. This is not hypothetical: `chardet`, an LGPL library
twelve years old, was relicensed to MIT with the help of a model, over the
objection of its original author.

The instinctive fix is to detect AI-authored code and refuse it. That does not
work, for the reasons already recorded in
[ADR 0002](0002-no-ai-authorship-detection.md), and it is also somebody else's
game: Black Duck and FOSSA have two decades of corpus and signature databases.
Competing there means losing there. Worse, detection output is probabilistic,
which under [ADR 0003](0003-evidence-classes.md) is *assessed* evidence and can
never bind. A detector could not gate anything even if it worked.

The way through is the observation that makes this tractable at all:

> **Copyrightability turns on human creative control, which is a fact about
> process, not about the artifact.**

You cannot read it off the code — which is exactly why detection fails, and
exactly why recording it at the moment of production works.

## Decision

**An authorship claim is `class: attested` Evidence carrying a structured
claim.** Not a sixth type; the model stays at five.

```rust
pub enum AuthorshipClaim {
    /// A human wrote it. Copyrightable by the declarer.
    Human,
    /// An agent produced it; a human directed and revised the expressive
    /// choices and asserts sufficient creative control.
    DirectedAgent,
    /// An agent produced it; no human claims creative control over the
    /// expressive elements. Likely uncopyrightable.
    Generated,
    /// Copied or adapted from an identified external source.
    Derived { source: String, license: Option<String> },
}
```

These map onto the USCO framework directly. `Generated` is the one that matters,
because it makes the previously invisible thing refusable **as policy**:

```yaml
require:
  authorship: [human, directed_agent]   # generated code cannot enter
```

A project that sets this keeps its licence enforceable, provably, with a signed
record behind every merged line.

**This is a declaration, not a detection, and the distinction is the whole
point.** It does not contradict ADR 0002. We are not inspecting source text and
guessing. A named human states something and signs it. If the statement is false,
they have signed a false statement — which is the same instrument as the DCO, and
the DCO works for exactly that reason.

## Consequences

**We cannot verify a claim is true.** Stated plainly because it is the first
objection anyone will raise. Neither can the DCO, and the DCO is the backbone of
kernel contribution. Enforcement is liability, not proof. What the record
provides is a named party, a signature, and a timestamp — which is precisely
what a court or an auditor needs and what nothing else produces.

**Granularity is a security property, not a UX detail.** If declaring `human`
across fifty files is one checkbox, it becomes a rubber stamp, and the more
capable the model the more likely that is. The declaration must be scoped
narrowly enough that making it falsely is uncomfortable. This is the hardest
open problem in the design and it is not solved by v0's per-contribution scope.

**Only works going forward.** Existing codebases cannot be reconstructed. That
is an argument for starting now, not an argument against.

**We are not lawyers.** The record is evidence, not a legal conclusion. Nothing
in this project should imply otherwise.

**It survives either legal outcome.** If courts eventually hold AI output *is*
copyrightable, the dilution mechanism weakens but the record does not become
worthless: the Copyright Office already requires disclosure of more-than-de-minimis
AI material at registration, and no tool produces that disclosure today.

## Alternatives rejected

**Detect and refuse.** Unwinnable, occupied by better-resourced incumbents, and
structurally unable to bind under our own rules.

**Ban AI-assisted contribution.** Unenforceable without detection, and wrong on
the merits — the goal is that the licence stays enforceable, not that a
particular tool goes unused.

**Require a CLA instead.** Solves the ownership question by taking assignment,
at the cost of a legal undertaking before a stranger's first contribution. This
project chose the DCO deliberately; an authorship claim is the same instrument
extended to the question the DCO does not ask.
