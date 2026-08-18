// SPDX-License-Identifier: Apache-2.0
//! The repository's own rules.
//!
//! GitLocus sets no bar of its own. A policy lives in the repository it governs,
//! is versioned with it, and is evaluated deterministically — the same policy,
//! contribution and evidence always produce the same verdict, whether that
//! evaluation happens on a laptop or in CI.

use crate::actor::TrustTier;
use crate::contribution::Contribution;
use crate::evidence::{Evidence, EvidenceClass, Outcome};
use crate::verdict::{Decision, Rank, Unmet, UnmetReason, Verdict};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Schema version of the policy document.
pub const POLICY_VERSION: u32 = 0;

/// Something wrong with a policy document.
#[derive(Debug)]
pub enum PolicyError {
    /// The document declares a schema version this build does not understand.
    UnsupportedVersion {
        /// The version the document declared.
        found: u32,
    },
    /// A path pattern failed to parse.
    BadGlob {
        /// The rule the pattern came from.
        rule: String,
        /// The offending pattern.
        pattern: String,
    },
    /// The document could not be parsed at all.
    Malformed(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => {
                write!(
                    f,
                    "unsupported policy version {found}, expected {POLICY_VERSION}"
                )
            }
            Self::BadGlob { rule, pattern } => {
                write!(f, "rule {rule} has an invalid path pattern {pattern}")
            }
            Self::Malformed(why) => write!(f, "malformed policy: {why}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Conditions under which a rule applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct When {
    /// Glob patterns matched against the contribution's changed paths.
    pub paths: Vec<String>,
}

/// What a rule demands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Require {
    /// Named checks that must be present, deterministic, and passing.
    #[serde(default)]
    pub deterministic: Vec<String>,
    /// How many human attestations are needed.
    #[serde(default)]
    pub approvals: u32,
    /// Minimum standing the contributor must hold.
    #[serde(default = "min_tier_default")]
    pub min_tier: TrustTier,
}

fn min_tier_default() -> TrustTier {
    TrustTier::Unknown
}

/// One rule: when it applies, and what it demands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name, reported in the verdict.
    pub name: String,
    /// When the rule applies.
    pub when: When,
    /// What it demands.
    pub require: Require,
}

/// A policy document, as written in `.gitlocus/policy.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Schema version. Must equal [`POLICY_VERSION`].
    pub version: u32,
    /// Rules, evaluated together. Every matching rule contributes.
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Parse a policy from YAML.
    ///
    /// # Errors
    /// Returns [`PolicyError::Malformed`] if the document is not valid YAML in
    /// the expected shape.
    pub fn from_yaml(src: &str) -> Result<Self, PolicyError> {
        serde_norway::from_str(src).map_err(|e| PolicyError::Malformed(e.to_string()))
    }

    /// Compile path patterns so that evaluation cannot fail.
    ///
    /// # Errors
    /// Returns [`PolicyError::UnsupportedVersion`] or [`PolicyError::BadGlob`].
    pub fn compile(self) -> Result<CompiledPolicy, PolicyError> {
        if self.version != POLICY_VERSION {
            return Err(PolicyError::UnsupportedVersion {
                found: self.version,
            });
        }
        let mut compiled = Vec::with_capacity(self.rules.len());
        for rule in self.rules {
            let mut builder = GlobSetBuilder::new();
            for pattern in &rule.when.paths {
                let glob = Glob::new(pattern).map_err(|_| PolicyError::BadGlob {
                    rule: rule.name.clone(),
                    pattern: pattern.clone(),
                })?;
                builder.add(glob);
            }
            let set = builder
                .build()
                .map_err(|e| PolicyError::Malformed(e.to_string()))?;
            compiled.push((rule, set));
        }
        Ok(CompiledPolicy { rules: compiled })
    }
}

/// A policy with its path patterns compiled. Evaluation is infallible.
#[derive(Debug)]
pub struct CompiledPolicy {
    rules: Vec<(Rule, GlobSet)>,
}

impl CompiledPolicy {
    /// Evaluate a contribution against this policy.
    ///
    /// Every rule whose paths match contributes: required checks are unioned, and
    /// approvals and minimum tier take the strictest value demanded. A change that
    /// touches both ordinary source and CI configuration is therefore held to the
    /// CI rule, which is the only safe way to combine the two.
    #[must_use]
    pub fn evaluate(&self, contribution: &Contribution, evidence: &[Evidence]) -> Verdict {
        let digest = contribution.head_digest.as_str();

        let mut matched = Vec::new();
        let mut required: BTreeSet<&str> = BTreeSet::new();
        let mut approvals_required = 0;
        let mut tier_required = TrustTier::Unknown;

        for (rule, globs) in &self.rules {
            if !contribution.changed_paths.iter().any(|p| globs.is_match(p)) {
                continue;
            }
            matched.push(rule.name.clone());
            required.extend(rule.require.deterministic.iter().map(String::as_str));
            approvals_required = approvals_required.max(rule.require.approvals);
            tier_required = tier_required.max(rule.require.min_tier);
        }

        let mut unmet = Vec::new();
        let mut satisfied = 0_u32;
        for kind in &required {
            match classify(kind, digest, evidence) {
                None => satisfied += 1,
                Some(reason) => unmet.push(Unmet {
                    requirement: (*kind).to_string(),
                    reason,
                }),
            }
        }

        let approvals_present = u32::try_from(
            evidence
                .iter()
                .filter(|e| {
                    e.class == EvidenceClass::Attested
                        && e.outcome == Outcome::Pass
                        && e.subject_digest == digest
                })
                .map(|e| e.produced_by.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX);

        let advisory = evidence
            .iter()
            .filter(|e| e.class == EvidenceClass::Assessed && e.subject_digest == digest)
            .map(|e| format!("{}: {:?}", e.kind, e.outcome))
            .collect();

        let tier_satisfied = contribution.actor.tier.satisfies(tier_required);
        let total = u32::try_from(required.len()).unwrap_or(u32::MAX);

        let decision = if !tier_satisfied || !unmet.is_empty() {
            Decision::Blocked
        } else if approvals_present < approvals_required {
            Decision::NeedsHuman
        } else {
            Decision::Satisfied
        };

        let confidence = if total == 0 {
            1.0
        } else {
            f64::from(satisfied) / f64::from(total)
        };

        Verdict {
            contribution: contribution.id(),
            subject_digest: digest.to_string(),
            decision,
            matched_rules: matched,
            unmet,
            approvals_required,
            approvals_present,
            tier_required,
            tier_satisfied,
            advisory,
            rank: Rank {
                confidence,
                human_cost: approvals_required.saturating_sub(approvals_present),
            },
        }
    }
}

/// Why a requirement is unmet, or `None` if it is met.
fn classify(kind: &str, digest: &str, evidence: &[Evidence]) -> Option<UnmetReason> {
    let candidates: Vec<&Evidence> = evidence.iter().filter(|e| e.kind == kind).collect();
    if candidates.is_empty() {
        return Some(UnmetReason::Missing);
    }
    if candidates.iter().any(|e| e.is_binding_for(digest)) {
        return None;
    }
    // Report the most actionable reason rather than whichever was seen first.
    if candidates
        .iter()
        .any(|e| e.outcome == Outcome::Fail && e.subject_digest == digest)
    {
        return Some(UnmetReason::Failed);
    }
    if candidates
        .iter()
        .any(|e| e.outcome == Outcome::Inconclusive && e.subject_digest == digest)
    {
        return Some(UnmetReason::Inconclusive);
    }
    if candidates.iter().all(|e| e.subject_digest != digest) {
        return Some(UnmetReason::StaleSubject);
    }
    Some(UnmetReason::NotDeterministic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Actor, ActorKind};

    const POLICY: &str = r#"
version: 0
rules:
  - name: baseline
    when:
      paths: ["**"]
    require:
      deterministic: [build, tests]
      approvals: 1
      min_tier: unknown
  - name: ci-config
    when:
      paths: [".github/workflows/**"]
    require:
      deterministic: [workflow-audit]
      approvals: 1
      min_tier: maintainer
"#;

    fn policy() -> CompiledPolicy {
        Policy::from_yaml(POLICY).unwrap().compile().unwrap()
    }

    fn contribution(paths: &[&str], tier: TrustTier) -> Contribution {
        Contribution {
            repository: "github.com/hey-vera/gitlocus".into(),
            base_digest: "base".into(),
            head_digest: "head".into(),
            actor: Actor {
                id: "someone".into(),
                kind: ActorKind::Human,
                tier,
                key_binding: None,
            },
            changed_paths: paths.iter().map(|s| (*s).to_string()).collect(),
            forge_ref: None,
        }
    }

    fn pass(kind: &str) -> Evidence {
        Evidence {
            kind: kind.into(),
            class: EvidenceClass::Deterministic,
            outcome: Outcome::Pass,
            subject_digest: "head".into(),
            produced_by: "ci".into(),
            produced_at: "2026-08-18T00:00:00Z".into(),
            source_uri: None,
            summary: None,
        }
    }

    fn approval(by: &str) -> Evidence {
        Evidence {
            kind: "review".into(),
            class: EvidenceClass::Attested,
            outcome: Outcome::Pass,
            subject_digest: "head".into(),
            produced_by: by.into(),
            produced_at: "2026-08-18T00:00:00Z".into(),
            source_uri: None,
            summary: None,
        }
    }

    #[test]
    fn missing_evidence_blocks() {
        let v = policy().evaluate(&contribution(&["src/main.rs"], TrustTier::Contributor), &[]);
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet.len(), 2);
    }

    #[test]
    fn all_checks_pass_but_approval_outstanding_needs_a_human() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests")],
        );
        assert_eq!(v.decision, Decision::NeedsHuman);
        assert_eq!(v.rank.human_cost, 1);
        assert!((v.rank.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn checks_plus_approval_satisfies() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests"), approval("maintainer-a")],
        );
        assert_eq!(v.decision, Decision::Satisfied);
        assert_eq!(v.rank.human_cost, 0);
    }

    #[test]
    fn duplicate_approvals_from_one_actor_count_once() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[
                pass("build"),
                pass("tests"),
                approval("same"),
                approval("same"),
            ],
        );
        assert_eq!(v.approvals_present, 1);
    }

    #[test]
    fn touching_ci_config_pulls_in_the_stricter_rule() {
        let c = contribution(
            &["src/main.rs", ".github/workflows/ci.yml"],
            TrustTier::Contributor,
        );
        let v = policy().evaluate(&c, &[pass("build"), pass("tests"), pass("workflow-audit")]);
        // Requirements unioned across both matched rules.
        assert_eq!(v.matched_rules.len(), 2);
        // Contributor does not clear the maintainer bar the CI rule demands.
        assert!(!v.tier_satisfied);
        assert_eq!(v.decision, Decision::Blocked);
    }

    #[test]
    fn assessed_evidence_never_satisfies_a_requirement() {
        let mut ai = pass("tests");
        ai.class = EvidenceClass::Assessed;
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), ai],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::NotDeterministic);
        // ...but it is still surfaced, because it may well be worth reading.
        assert_eq!(v.advisory.len(), 1);
    }

    #[test]
    fn evidence_for_a_previous_revision_is_reported_as_stale() {
        let mut old = pass("tests");
        old.subject_digest = "an-older-revision".into();
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), old],
        );
        assert_eq!(v.unmet[0].reason, UnmetReason::StaleSubject);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);
        let ev = [pass("build"), pass("tests")];
        let a = policy().evaluate(&c, &ev);
        let b = policy().evaluate(&c, &ev);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let src = "version: 99\nrules: []\n";
        let err = Policy::from_yaml(src).unwrap().compile().unwrap_err();
        assert!(matches!(err, PolicyError::UnsupportedVersion { found: 99 }));
    }
}
