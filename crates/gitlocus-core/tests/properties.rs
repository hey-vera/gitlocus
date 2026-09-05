// SPDX-License-Identifier: Apache-2.0
//! The invariants that are universally quantified, tested as properties.
//!
//! Four of the seven rules in AGENTS.md are statements about *every* input, and
//! four were tested on inputs somebody chose. That is not a criticism of those
//! tests — a hand-written case is how you show a reader what the rule means, and
//! `clause_5_evidence_order_does_not_change_the_verdict` does that well. It is a
//! statement about what an example can establish: an example establishes that
//! the rule holds for the example.
//!
//! Where the claim is "for all", the test should quantify over all. See
//! [ADR 0018](../../../docs/adr/0018-quantified-claims-are-tested-as-properties.md).

use gitlocus_core::policy::CompiledPolicy;
use gitlocus_core::{
    Actor, ActorKind, Contribution, Delegation, Evidence, EvidenceClass, Outcome, Policy,
    TrustTier, VouchList,
};
use proptest::prelude::*;

// --- generators ----------------------------------------------------------------

/// Digests are compared, never interpreted, so a short alphabet exercises the
/// comparison far more densely than realistic SHA-256 strings would: with four
/// possible values, evidence binding to the head is a case that actually occurs.
fn digest() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["aaaa", "bbbb", "cccc", "dddd"]).prop_map(str::to_string)
}

fn evidence_class() -> impl Strategy<Value = EvidenceClass> {
    prop_oneof![
        Just(EvidenceClass::Deterministic),
        Just(EvidenceClass::Attested),
        Just(EvidenceClass::Assessed),
    ]
}

fn outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        Just(Outcome::Pass),
        Just(Outcome::Fail),
        Just(Outcome::Inconclusive),
    ]
}

fn evidence() -> impl Strategy<Value = Evidence> {
    (
        prop::sample::select(vec!["build", "tests", "lint", "approval"]),
        evidence_class(),
        outcome(),
        digest(),
        prop::sample::select(vec!["ci", "someone", "an-agent"]),
    )
        .prop_map(
            |(kind, class, outcome, subject_digest, produced_by)| Evidence {
                kind: kind.to_string(),
                class,
                outcome,
                subject_digest,
                produced_by: produced_by.to_string(),
                produced_at: "2026-08-19T00:00:00Z".to_string(),
                source_uri: None,
                summary: None,
                authorship: None,
                signer: None,
            },
        )
}

fn contribution() -> impl Strategy<Value = Contribution> {
    (
        digest(),
        digest(),
        prop::collection::vec(
            prop::sample::select(vec!["src/lib.rs", ".github/workflows/ci.yml", "README.md"]),
            0..4,
        ),
    )
        .prop_map(|(base, head, paths)| Contribution {
            repository: "github.com/acme/repo".to_string(),
            base_digest: base,
            head_digest: head,
            actor: Actor {
                id: "someone".to_string(),
                kind: ActorKind::Human,
                tier: TrustTier::Contributor,
                key_binding: None,
                delegation: Vec::new(),
            },
            changed_paths: paths.into_iter().map(str::to_string).collect(),
            forge_ref: None,
        })
}

/// The policy this repository runs on itself, which is the one whose behaviour
/// anyone can check against a file in the tree.
fn own_policy() -> Policy {
    Policy::from_yaml(include_str!("../../../.gitlocus/policy.yml")).expect("this repo's policy")
}

// --- invariant 4: verdicts are pure --------------------------------------------

proptest! {
    /// AGENTS.md invariant 4: "no dependence on the order of the evidence array."
    ///
    /// `clause_5_evidence_order_does_not_change_the_verdict` permutes one
    /// hand-written array of three records. The claim is about any permutation
    /// of any array, and it is the claim the swarm-scale caching argument rests
    /// on: a verdict that depends on the order its inputs arrived in is not
    /// content-addressable, whatever else it is.
    #[test]
    fn no_permutation_of_the_evidence_changes_the_verdict(
        contribution in contribution(),
        evidence in prop::collection::vec(evidence(), 0..8),
        shuffle in prop::collection::vec(0usize..8, 0..24),
    ) {
        let compiled = own_policy().compile().expect("compiles");
        let expected = compiled.evaluate(&contribution, &evidence);

        // A sequence of swaps rather than a single reversal: reversal is one
        // permutation, and the claim is about all of them.
        let mut permuted = evidence.clone();
        if !permuted.is_empty() {
            for pair in shuffle.chunks(2) {
                if let [a, b] = pair {
                    let (a, b) = (a % permuted.len(), b % permuted.len());
                    permuted.swap(a, b);
                }
            }
        }

        let actual = compiled.evaluate(&contribution, &permuted);
        prop_assert_eq!(expected, actual);
    }

    /// Invariant 4 again, from the other side: the same inputs twice must give
    /// byte-identical answers. A clock or any ambient state reachable from
    /// `evaluate` shows up here.
    #[test]
    fn evaluating_twice_gives_the_same_verdict(
        contribution in contribution(),
        evidence in prop::collection::vec(evidence(), 0..8),
    ) {
        let compiled = own_policy().compile().expect("compiles");
        let first = compiled.evaluate(&contribution, &evidence);
        let second = compiled.evaluate(&contribution, &evidence);
        prop_assert_eq!(
            serde_json::to_string(&first).expect("serialise"),
            serde_json::to_string(&second).expect("serialise")
        );
    }

    /// Invariant 1, quantified: assessed evidence must never satisfy a
    /// requirement — "not with a high score, not with a confidence threshold,
    /// not behind a flag".
    ///
    /// Deleting every assessed record from the input must not change the
    /// verdict. If assessed evidence could ever contribute, some generated case
    /// makes the two differ.
    #[test]
    fn removing_every_assessed_record_never_changes_the_verdict(
        contribution in contribution(),
        evidence in prop::collection::vec(evidence(), 0..8),
    ) {
        let compiled = own_policy().compile().expect("compiles");
        let with = compiled.evaluate(&contribution, &evidence);

        let without: Vec<Evidence> = evidence
            .iter()
            .filter(|e| e.class != EvidenceClass::Assessed)
            .cloned()
            .collect();
        let got = compiled.evaluate(&contribution, &without);

        prop_assert_eq!(with.decision, got.decision);
    }

    /// Invariant 2, quantified: evidence bound to a different revision must
    /// never count.
    ///
    /// Records whose `subject_digest` is not the head digest may be dropped
    /// without changing the decision. This is what stops a green result from
    /// before a force-push being credited to the code that replaced it.
    #[test]
    fn evidence_for_another_revision_never_changes_the_decision(
        contribution in contribution(),
        evidence in prop::collection::vec(evidence(), 0..8),
    ) {
        let compiled = own_policy().compile().expect("compiles");
        let with = compiled.evaluate(&contribution, &evidence);

        let bound: Vec<Evidence> = evidence
            .iter()
            .filter(|e| e.subject_digest == contribution.head_digest)
            .cloned()
            .collect();
        let got = compiled.evaluate(&contribution, &bound);

        prop_assert_eq!(with.decision, got.decision);
    }
}

// --- merging is monotone --------------------------------------------------------

proptest! {
    /// `merged` documents that requirements are unioned and approvals and tier
    /// take the strictest value, so "a change touching both src and CI config is
    /// held to the CI rule". The property behind that sentence is monotonicity:
    /// adding a policy can only ever make the outcome stricter, never looser.
    ///
    /// This is what makes ADR 0013 safe. The gate evaluates the base policy
    /// together with the head policy precisely so that a contribution cannot
    /// weaken what governs it — and that only holds if merging is monotone.
    #[test]
    fn adding_a_policy_never_loosens_the_decision(
        contribution in contribution(),
        evidence in prop::collection::vec(evidence(), 0..8),
    ) {
        let base = own_policy();
        let extra = Policy::from_yaml(
            r#"
version: 0
rules:
  - name: stricter
    when:
      paths: ["**"]
    require:
      deterministic: [something-nobody-produces]
      approvals: 3
      min_tier: maintainer
"#,
        )
        .expect("parses");

        let base = base.compile().expect("compiles");
        let extra = extra.compile().expect("compiles");

        let alone = base.evaluate(&contribution, &evidence);
        let merged = CompiledPolicy::merged(vec![base, extra])
            .evaluate(&contribution, &evidence);

        // `is_mergeable` is the property that matters: merging in a policy that
        // demands more must never turn a blocked contribution into a passing one.
        prop_assert!(
            !merged.is_mergeable() || alone.is_mergeable(),
            "merging a stricter policy made a non-mergeable contribution mergeable"
        );
    }
}

// --- parsers are total ----------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// `vouch.rs` states it outright: "Parsing is total: a line this module
    /// cannot interpret is ignored rather than rejected. A trust file that fails
    /// closed on a typo would be a denial of service on the project that owns
    /// it."
    ///
    /// A total parser is the canonical property: for all inputs, it returns.
    /// Twelve hand-written cases cannot establish that, and `VOUCHED.td` is read
    /// out of a contribution's own tree, so its content is chosen by whoever
    /// opened the pull request.
    #[test]
    fn parsing_a_vouch_file_never_panics(src in ".*") {
        let list = VouchList::parse(&src);
        // Querying it must be total too — a panic in `status` is reachable from
        // exactly the same input.
        let _ = list.status("github", "someone");
        let _ = list.len();
    }

    /// The same property for the policy parser, which reads YAML out of a
    /// contribution's own revision (ADR 0013). Failing is fine and expected;
    /// panicking is not, because the gate is what would panic.
    #[test]
    fn parsing_a_policy_never_panics(src in ".*") {
        if let Ok(policy) = Policy::from_yaml(&src) {
            // Compiling is where globs are built, and a glob out of untrusted
            // YAML is the part with something to get wrong.
            let _ = policy.compile();
        }
    }
}

// --- ADR 0007: attenuation, quantified over every chain -------------------------

fn tier() -> impl Strategy<Value = TrustTier> {
    prop_oneof![
        Just(TrustTier::Unknown),
        Just(TrustTier::Vouched),
        Just(TrustTier::Contributor),
        Just(TrustTier::Maintainer),
    ]
}

fn chain() -> impl Strategy<Value = Vec<Delegation>> {
    prop::collection::vec(
        (tier(), prop::option::of(Just("grant".to_string()))).prop_map(|(ceiling, grant)| {
            Delegation {
                delegator: "someone".to_string(),
                ceiling,
                grant,
            }
        }),
        0..4,
    )
}

fn answerable_kind() -> impl Strategy<Value = ActorKind> {
    prop_oneof![
        Just(ActorKind::Human),
        Just(ActorKind::Pair {
            implementation: "claude-code".to_string(),
            model: None,
            operator: "josh".to_string(),
        }),
    ]
}

proptest! {
    /// ADR 0007: "a delegated actor may never hold a tier above the actor that
    /// delegated to it." For every root tier and every chain, the effective
    /// tier is at most the root's and at most every ceiling's — a hop can only
    /// lower the result. Conformance clause 11.
    #[test]
    fn effective_tier_never_exceeds_the_root_or_any_ceiling(
        root in tier(),
        kind in answerable_kind(),
        delegation in chain(),
    ) {
        let actor = Actor {
            id: "josh".to_string(),
            kind,
            tier: root,
            key_binding: None,
            delegation: delegation.clone(),
        };
        let effective = actor.effective_tier();
        prop_assert!(effective <= root, "{effective:?} above the root {root:?}");
        for hop in &delegation {
            prop_assert!(effective <= hop.ceiling, "{effective:?} above a ceiling {:?}", hop.ceiling);
        }
        // And no lower than it has to be: the minimum, exactly.
        let expected = delegation.iter().fold(root, |t, hop| t.min(hop.ceiling));
        prop_assert_eq!(effective, expected);
    }

    /// ADR 0007: "a chain with no human at its root terminates at unknown."
    /// Whatever tier the document asserts and whatever the ceilings say, an
    /// agent nobody answers for holds no standing. Conformance clause 11.
    #[test]
    fn an_agent_rooted_chain_is_always_unknown(root in tier(), delegation in chain()) {
        let actor = Actor {
            id: "agent".to_string(),
            kind: ActorKind::Agent {
                implementation: "claude-code".to_string(),
                model: Some("some-model".to_string()),
            },
            tier: root,
            key_binding: None,
            delegation,
        };
        prop_assert_eq!(actor.effective_tier(), TrustTier::Unknown);
    }
}
