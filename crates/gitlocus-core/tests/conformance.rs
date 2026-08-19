// SPDX-License-Identifier: Apache-2.0
//! Conformance: the implementation and the published schemas must agree.
//!
//! Section 6 of the specification is a list of claims. This file is the
//! executable form of the ones a test can settle. It also validates that what
//! this crate actually serialises matches the schemas in `spec/schemas`, which
//! is the failure mode a specification repository most often ships: prose and
//! code that drifted apart between releases.

use gitlocus_core::policy::{CompiledPolicy, Policy};
use gitlocus_core::verdict::{Decision, UnmetReason};
use gitlocus_core::{Actor, ActorKind, Contribution, Evidence, EvidenceClass, Outcome, TrustTier};
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
        changed_paths: vec!["crates/gitlocus-core/src/policy.rs".into()],
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

/// This repository's own policy, as the source it is written in.
fn own_policy_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.gitlocus/policy.yml");
    std::fs::read_to_string(&path).expect("this repository must carry its own policy")
}

#[test]
fn clause_4_matching_rules_union_and_take_the_strictest() {
    // A contribution touching ordinary source and the policy at once is held to
    // the policy rule, not to whichever matched first. Otherwise a contributor
    // weakens the rule governing them by also touching a leniently governed file.
    let mut c = sample_contribution();
    c.changed_paths = vec![
        "crates/gitlocus-core/src/policy.rs".into(),
        ".gitlocus/policy.yml".into(),
    ];
    c.actor.tier = TrustTier::Contributor;

    let v = Policy::from_yaml(&own_policy_src())
        .unwrap()
        .compile()
        .unwrap()
        .evaluate(&c, &[]);

    assert_eq!(
        v.tier_required,
        TrustTier::Maintainer,
        "strictest tier demanded"
    );
    assert!(!v.tier_satisfied);
    assert!(
        v.unmet.iter().any(|u| u.requirement == "workflow-audit"),
        "requirements unioned across every matching rule, not just the first"
    );
}

#[test]
fn clause_6_a_contribution_cannot_weaken_the_policy_that_governs_it() {
    // The clause that had no executable form, and the reason it needed one: the
    // gate read only the policy the pull request shipped, so a change deleting
    // every rule was judged by a document containing no rules, and came back
    // satisfied with no evidence and no standing.
    let mut c = sample_contribution();
    c.changed_paths = vec![".gitlocus/policy.yml".into()];
    c.actor.tier = TrustTier::Unknown;

    let gutted = || {
        Policy::from_yaml("version: 0\nrules: []\n")
            .unwrap()
            .compile()
            .unwrap()
    };

    let shipped = gutted().evaluate(&c, &[]);
    assert_eq!(
        shipped.decision,
        Decision::Satisfied,
        "the failure being closed: judged by what it ships, it demands nothing of itself"
    );

    let governing = Policy::from_yaml(&own_policy_src())
        .unwrap()
        .labelled("governing")
        .compile()
        .unwrap();
    let governed = CompiledPolicy::merged(vec![governing, gutted()]).evaluate(&c, &[]);

    assert_eq!(governed.decision, Decision::Blocked);
    assert!(!governed.tier_satisfied);
    assert!(
        governed
            .matched_rules
            .iter()
            .any(|r| r == "governing:ci-and-policy"),
        "the verdict must name the document the blocking rule came from"
    );
}

#[test]
fn clause_6_a_policy_that_cannot_be_read_is_not_the_same_as_one_that_is_absent() {
    // Absence is legitimate — a first adoption has no policy at the base
    // revision. A read or parse failure is not, and must not degrade into it.
    assert!(Policy::from_yaml("version: 0\nrules: [").is_err());
    assert!(
        Policy::from_yaml("version: 99\nrules: []\n")
            .unwrap()
            .compile()
            .is_err()
    );
}

#[test]
fn clause_7_a_signer_is_never_read_from_input() {
    // Section 6 clause 7. The whole value of a signed_by rule rests on this:
    // if a document can name its own signer, forging trusted CI identity is a
    // matter of typing it into a file.
    let forged = r#"{
        "kind": "tests",
        "class": "deterministic",
        "outcome": "pass",
        "subject_digest": "bbbb2222",
        "produced_by": "someone",
        "produced_at": "2026-08-18T12:00:00Z",
        "signer": "https://github.com/hey-vera/gitlocus/.github/workflows/ci.yml@refs/heads/main"
    }"#;
    let parsed: Evidence = serde_json::from_str(forged).unwrap();
    assert_eq!(parsed.signer, None);
}

#[test]
fn clause_8_unsigned_evidence_cannot_satisfy_a_signed_requirement() {
    let policy = Policy::from_yaml(SIGNED).unwrap().compile().unwrap();
    let mut e = evidence("tests", EvidenceClass::Deterministic, Outcome::Pass);
    e.signer = None;
    let v = policy.evaluate(&sample_contribution(), &[e]);
    assert_eq!(
        v.unmet.first().map(|u| u.reason),
        Some(UnmetReason::Unsigned)
    );
}

#[test]
fn clause_9_a_signature_from_the_wrong_identity_cannot_satisfy_it() {
    let policy = Policy::from_yaml(SIGNED).unwrap().compile().unwrap();
    let mut e = evidence("tests", EvidenceClass::Deterministic, Outcome::Pass);
    e.signer =
        Some("https://github.com/impostor/repo/.github/workflows/ci.yml@refs/heads/main".into());
    let v = policy.evaluate(&sample_contribution(), &[e]);
    assert_eq!(
        v.unmet.first().map(|u| u.reason),
        Some(UnmetReason::WrongSigner)
    );
}

/// A policy demanding that the tests check be signed by this repository's CI.
const SIGNED: &str = r#"
version: 0
rules:
  - name: signed
    when:
      paths: ["**"]
    require:
      deterministic: [tests]
      approvals: 0
      signed_by:
        tests: "https://github.com/hey-vera/gitlocus/.github/workflows/*@*"
"#;
