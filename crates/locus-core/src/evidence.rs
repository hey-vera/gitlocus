// SPDX-License-Identifier: Apache-2.0
//! Claims about a contribution, and how much each claim is worth.

use serde::{Deserialize, Serialize};

/// How much a claim is worth, and therefore how it may be used.
///
/// This is the distinction the whole system turns on. A test suite exiting zero
/// and a language model reporting "looks fine" are both claims about a change,
/// but only one of them can be re-run to the same answer by a third party.
/// Rendering them alike is what turns a review queue into a coin flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// Reproducible by anyone with the inputs: exit codes, digests, test results.
    /// Only this class may satisfy a blocking requirement.
    Deterministic,
    /// A judgement produced by a heuristic or a model. Informative, never binding.
    Assessed,
    /// A human took responsibility. Cannot be produced by automation.
    Attested,
}

/// The result a piece of evidence reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The check succeeded.
    Pass,
    /// The check failed.
    Fail,
    /// The check could not reach an answer. Treated as unmet, never as a pass.
    Inconclusive,
}

/// A single claim about a contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Requirement name this evidence answers to, e.g. `tests` or `clippy`.
    pub kind: String,
    /// How much the claim is worth.
    pub class: EvidenceClass,
    /// What it reports.
    pub outcome: Outcome,
    /// Digest of the revision the claim was made about.
    pub subject_digest: String,
    /// Identifier of whatever produced the claim.
    pub produced_by: String,
    /// RFC 3339 timestamp.
    pub produced_at: String,
    /// Where the underlying run can be inspected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// Free-form detail. Never parsed for control flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl Evidence {
    /// Whether this evidence can satisfy a blocking requirement.
    ///
    /// Assessed evidence never can, however confident it sounds, and evidence
    /// about a different revision never can either — that last case is how stale
    /// green checks get reused across a force-push.
    #[must_use]
    pub fn is_binding_for(&self, subject_digest: &str) -> bool {
        self.class == EvidenceClass::Deterministic
            && self.outcome == Outcome::Pass
            && self.subject_digest == subject_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(class: EvidenceClass, outcome: Outcome, digest: &str) -> Evidence {
        Evidence {
            kind: "tests".into(),
            class,
            outcome,
            subject_digest: digest.into(),
            produced_by: "ci".into(),
            produced_at: "2026-08-18T00:00:00Z".into(),
            source_uri: None,
            summary: None,
        }
    }

    #[test]
    fn only_deterministic_passes_bind() {
        assert!(ev(EvidenceClass::Deterministic, Outcome::Pass, "abc").is_binding_for("abc"));
        assert!(!ev(EvidenceClass::Assessed, Outcome::Pass, "abc").is_binding_for("abc"));
        assert!(!ev(EvidenceClass::Deterministic, Outcome::Fail, "abc").is_binding_for("abc"));
        assert!(
            !ev(EvidenceClass::Deterministic, Outcome::Inconclusive, "abc").is_binding_for("abc")
        );
    }

    #[test]
    fn evidence_from_another_revision_does_not_bind() {
        assert!(!ev(EvidenceClass::Deterministic, Outcome::Pass, "old").is_binding_for("new"));
    }
}
