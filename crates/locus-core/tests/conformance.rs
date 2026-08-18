// SPDX-License-Identifier: Apache-2.0
//! Conformance: the implementation and the published schemas must agree.
//!
//! Section 6 of the specification is a list of claims. This file is the
//! executable form of the ones a test can settle. It also validates that what
//! this crate actually serialises matches the schemas in `spec/schemas`, which
//! is the failure mode a specification repository most often ships: prose and
//! code that drifted apart between releases.

use locus_core::policy::Policy;
use locus_core::{Actor, ActorKind, Contribution, Evidence, EvidenceClass, Outcome, TrustTier};
use std::path::PathBuf;

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/schemas")
}

fn schema(name: &str) -> serde_json::Value {
    let path = spec_dir().join(name);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&src).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn assert_valid(schema_name: &str, instance: &serde_json::Value) {
    let schema = schema(schema_name);
    let validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|e| panic!("{schema_name} is not a valid JSON Schema: {e}"));
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "{schema_name} rejected our own output: {errors:?}"
    );
}

fn sample_contribution() -> Contribution {
    Contribution {
        repository: "github.com/hey-vera/gitlocus".into(),
        base_digest: "aaaa1111".into(),
        head_digest: "bbbb2222".into(),
        actor: Actor {
            id: "claude-opus-5".into(),
            kind: ActorKind::Pair {
                implementation: "claude-code".into(),
                operator: "josh".into(),
            },
            tier: TrustTier::Contributor,
            key_binding: Some("https://token.actions.githubusercontent.com".into()),
        },
        changed_paths: vec!["crates/locus-core/src/policy.rs".into()],
        forge_ref: Some("https://github.com/hey-vera/gitlocus/pull/1".into()),
    }
}

fn evidence(kind: &str, class: EvidenceClass, outcome: Outcome) -> Evidence {
    Evidence {
        kind: kind.into(),
        class,
        outcome,
        subject_digest: "bbbb2222".into(),
        produced_by: "github-actions".into(),
        produced_at: "2026-08-18T12:00:00Z".into(),
        source_uri: Some("https://github.com/hey-vera/gitlocus/actions/runs/1".into()),
        summary: None,
        signer: None,
    }
}

#[test]
fn serialised_contribution_matches_schema() {
    let json = serde_json::to_value(sample_contribution()).unwrap();
    assert_valid("contribution.schema.json", &json);
}

#[test]
fn serialised_evidence_matches_schema() {
    for class in [
        EvidenceClass::Deterministic,
        EvidenceClass::Assessed,
        EvidenceClass::Attested,
    ] {
        for outcome in [Outcome::Pass, Outcome::Fail, Outcome::Inconclusive] {
            let json = serde_json::to_value(evidence("tests", class, outcome)).unwrap();
            assert_valid("evidence.schema.json", &json);
        }
    }
}

#[test]
fn the_repositorys_own_policy_matches_schema() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.gitlocus/policy.yml");
    let src = std::fs::read_to_string(&path).expect("this repository must carry its own policy");
    let policy = Policy::from_yaml(&src).expect("own policy must parse");
    let json = serde_json::to_value(&policy).unwrap();
    assert_valid("policy.schema.json", &json);
    policy.compile().expect("own policy must compile");
}

#[test]
fn serialised_verdict_matches_schema() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.gitlocus/policy.yml");
    let policy = Policy::from_yaml(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .compile()
        .unwrap();
    let verdict = policy.evaluate(
        &sample_contribution(),
        &[evidence(
            "build",
            EvidenceClass::Deterministic,
            Outcome::Pass,
        )],
    );
    let json = serde_json::to_value(&verdict).unwrap();
    assert_valid("verdict.schema.json", &json);
}

// --- Specification section 6, clause by clause -----------------------------

#[test]
fn clause_1_assessed_evidence_cannot_satisfy_a_requirement() {
    assert!(
        !evidence("build", EvidenceClass::Assessed, Outcome::Pass).is_binding_for("bbbb2222"),
        "assessed evidence must never bind, however confident it sounds"
    );
}

#[test]
fn clause_2_evidence_for_another_revision_cannot_satisfy_a_requirement() {
    let mut stale = evidence("build", EvidenceClass::Deterministic, Outcome::Pass);
    stale.subject_digest = "an-older-revision".into();
    assert!(!stale.is_binding_for("bbbb2222"));
}

#[test]
fn clause_3_inconclusive_is_unmet_not_passed() {
    assert!(
        !evidence("build", EvidenceClass::Deterministic, Outcome::Inconclusive)
            .is_binding_for("bbbb2222")
    );
}

#[test]
fn clause_5_identical_inputs_produce_byte_identical_verdicts() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.gitlocus/policy.yml");
    let src = std::fs::read_to_string(path).unwrap();
    let contribution = sample_contribution();
    let ev = [
        evidence("build", EvidenceClass::Deterministic, Outcome::Pass),
        evidence("tests", EvidenceClass::Deterministic, Outcome::Pass),
    ];

    // Compiled separately on purpose: a shared compiled policy could hide state.
    let first = Policy::from_yaml(&src)
        .unwrap()
        .compile()
        .unwrap()
        .evaluate(&contribution, &ev);
    let second = Policy::from_yaml(&src)
        .unwrap()
        .compile()
        .unwrap()
        .evaluate(&contribution, &ev);

    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn clause_5_evidence_order_does_not_change_the_verdict() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.gitlocus/policy.yml");
    let src = std::fs::read_to_string(path).unwrap();
    let policy = || Policy::from_yaml(&src).unwrap().compile().unwrap();
    let contribution = sample_contribution();

    let forward = [
        evidence("build", EvidenceClass::Deterministic, Outcome::Pass),
        evidence("tests", EvidenceClass::Deterministic, Outcome::Pass),
        evidence("lint", EvidenceClass::Assessed, Outcome::Fail),
    ];
    let mut reversed = forward.clone();
    reversed.reverse();

    assert_eq!(
        serde_json::to_string(&policy().evaluate(&contribution, &forward)).unwrap(),
        serde_json::to_string(&policy().evaluate(&contribution, &reversed)).unwrap()
    );
}
