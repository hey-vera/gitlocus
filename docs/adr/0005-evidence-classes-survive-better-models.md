<!-- SPDX-License-Identifier: Apache-2.0 -->
# ADR 0005 — Evidence classes survive better models

- **Status:** accepted
- **Date:** 2026-08-18
- **Supersedes nothing. Reinforces [ADR 0003](0003-evidence-classes.md).**

## Context

This decision exists because one question will be asked of this project
repeatedly, by well-meaning people, for as long as it lives:

> Models are far more reliable now than when you wrote that rule. Surely
> assessed evidence should be allowed to bind, at least above some confidence?

The question sounds reasonable and gets more persuasive every year. It is the
single most likely way this project loses the property that makes it worth
having. Writing the answer down once, in a place that outlives any particular
conversation, is the point of this record.

## Decision

**No. Not at any confidence, not behind a flag, not for any model, ever.**

The class distinction is **not** a claim about reliability. If it were, it would
weaken as models improve, and the question above would eventually be right.

It is a claim about three properties that do not improve with model capability:

**1. Reproducibility.** A deterministic check can be re-run by a third party with
the same inputs and will produce the same answer. A model's judgement cannot —
not because it is unreliable, but because re-running it is a fresh sample from a
distribution, and because the model that produced the original answer may not
exist by the time anyone wants to check it. A verdict nobody can re-derive is not
auditable, however good it was.

**2. Injectability.** A model reviewing a contribution reads that contribution,
and the contribution is written by the party being evaluated. That is a direct,
unavoidable adversarial channel. Prompt injection is not a defect of current
models that better training removes — it is a structural consequence of taking
instructions and data through the same channel. A *more* capable model follows a
successful injection *more* competently.

**3. Liability.** A deterministic check is a fact. A human attestation is a
person accepting responsibility. A model's judgement is neither: there is no
party to hold accountable when it is wrong. Accountability is the mechanism that
actually makes the system work — the DCO functions not because anyone verifies
it but because a signed false statement is a liability — and a model cannot bear
it. A perfect oracle you cannot re-run, cannot audit, and cannot sue is still
not a deterministic check.

None of these three erodes as models improve. Therefore the rule does not.

## Consequences

**The most valuable finding on a pull request may be non-binding.** That is
correct and worth restating: assessed evidence is often the most *useful* thing
present. It is surfaced in `Verdict::advisory` precisely so a human reads it. Not
binding is not the same as not valuable.

**Prompt injection stays bounded by construction.** The worst outcome of a
successful injection against a model reviewer is a misleading advisory note. Not
a merge. This is the strongest security property the project has, and it exists
only because of this rule.

**We give up auto-merge on model judgement.** A real cost, deliberately paid.
The counter-argument — that today's models are good enough — misses that a
structural guarantee which holds only while the model behaves is not a guarantee.

**What may change as models improve:** the *meaning* of `attested` shifts. Today
a human attestation often means "I read this and it looks right". In a world
where machines review better than people, it will mean "I accept responsibility
for this". That shift is fine. The class survives because accountability, not
inspection, was always what it encoded.

## If you are here to change this

You are welcome to argue with it — open a discussion. But the argument has to
engage the three properties above, not the reliability of the current best model.
"The model is very good now" is not a response to any of them.

The one argument that *would* move this: a mechanism by which a model's judgement
becomes independently reproducible by a third party years later, and by which
some legal person accepts liability for it. If that exists, this record should be
revisited. It does not exist today.
