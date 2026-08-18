// SPDX-License-Identifier: Apache-2.0
//! The output: what a change still needs, and where it belongs in the queue.

use crate::actor::TrustTier;
use serde::{Deserialize, Serialize};

/// What should happen to a contribution next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Everything the policy asks for is present. Ready to merge.
    Satisfied,
    /// Machines are done and agree; a human still has to take responsibility.
    NeedsHuman,
    /// Something required is missing, failing, or the contributor lacks standing.
    /// No human attention is warranted yet.
    Blocked,
}

impl Decision {
    /// Sort order for a review queue: satisfied work first, blocked work last.
    ///
    /// Blocked changes sort last on purpose. They are the ones a maintainer
    /// cannot act on, and putting them in front of work that is ready is the
    /// specific failure this project exists to fix.
    #[must_use]
    pub fn queue_order(self) -> u8 {
        match self {
            Self::Satisfied => 0,
            Self::NeedsHuman => 1,
            Self::Blocked => 2,
        }
    }
}

/// Why a required check is not satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnmetReason {
    /// No evidence of this kind was submitted at all.
    Missing,
    /// Evidence was submitted and it failed.
    Failed,
    /// The check ran but could not reach an answer.
    Inconclusive,
    /// Evidence exists but is assessed rather than deterministic, so it cannot bind.
    NotDeterministic,
    /// Evidence exists but describes a different revision — typically a green
    /// result carried over from before a force-push.
    StaleSubject,
    /// The policy demands a verified signature and the evidence carries none.
    ///
    /// Distinct from [`UnmetReason::Missing`] on purpose: the check ran and
    /// reported a pass, but nothing establishes who produced that claim, so it
    /// is a claim rather than a fact.
    Unsigned,
    /// Evidence is signed, but not by an identity the policy accepts.
    ///
    /// This is what stops a passing result produced on a laptop from standing in
    /// for one produced by the repository's own CI.
    WrongSigner,
}

/// One unsatisfied requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unmet {
    /// The requirement name from the policy.
    pub requirement: String,
    /// Why it is unsatisfied.
    pub reason: UnmetReason,
}

/// Signals for ordering a queue of contributions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rank {
    /// Fraction of required deterministic checks that are satisfied, 0.0 to 1.0.
    pub confidence: f64,
    /// How many human attestations are still outstanding.
    pub human_cost: u32,
}

/// The reproducible result of evaluating a contribution against a policy.
///
/// Two evaluations of the same policy, contribution and evidence produce byte
/// identical verdicts. That is what lets a contributor run the gate locally and
/// know what CI will say.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    /// Forge-independent identity of the contribution.
    pub contribution: String,
    /// Revision the verdict describes.
    pub subject_digest: String,
    /// What should happen next.
    pub decision: Decision,
    /// Names of the policy rules that matched.
    pub matched_rules: Vec<String>,
    /// Requirements that are not satisfied.
    pub unmet: Vec<Unmet>,
    /// Human attestations the policy demands.
    pub approvals_required: u32,
    /// Distinct human attestations present for this revision.
    pub approvals_present: u32,
    /// Minimum standing the policy demands.
    pub tier_required: TrustTier,
    /// Whether the contributor clears it.
    pub tier_satisfied: bool,
    /// Assessed findings, surfaced for the reader and binding on nothing.
    pub advisory: Vec<String>,
    /// Queue-ordering signals.
    pub rank: Rank,
}

impl Verdict {
    /// Whether the contribution may merge under this policy.
    #[must_use]
    pub fn is_mergeable(&self) -> bool {
        self.decision == Decision::Satisfied
    }

    /// Process exit code: zero when mergeable, non-zero otherwise.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        i32::from(!self.is_mergeable())
    }

    /// A short human-readable line summarising the verdict.
    #[must_use]
    pub fn headline(&self) -> String {
        match self.decision {
            Decision::Satisfied => "satisfied: every requirement met".to_string(),
            Decision::NeedsHuman => format!(
                "needs a human: {} of {} approvals present",
                self.approvals_present, self.approvals_required
            ),
            Decision::Blocked if !self.tier_satisfied => {
                format!(
                    "blocked: contributor does not clear {:?}",
                    self.tier_required
                )
            }
            Decision::Blocked => format!("blocked: {} unmet requirement(s)", self.unmet.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_work_sorts_ahead_of_blocked_work() {
        let mut order = [Decision::Blocked, Decision::Satisfied, Decision::NeedsHuman];
        order.sort_by_key(|d| d.queue_order());
        assert_eq!(
            order,
            [Decision::Satisfied, Decision::NeedsHuman, Decision::Blocked]
        );
    }

    #[test]
    fn exit_code_is_zero_only_when_mergeable() {
        let base = Verdict {
            contribution: "r@a..b".into(),
            subject_digest: "b".into(),
            decision: Decision::Satisfied,
            matched_rules: vec![],
            unmet: vec![],
            approvals_required: 0,
            approvals_present: 0,
            tier_required: TrustTier::Unknown,
            tier_satisfied: true,
            advisory: vec![],
            rank: Rank {
                confidence: 1.0,
                human_cost: 0,
            },
        };
        assert_eq!(base.exit_code(), 0);

        let blocked = Verdict {
            decision: Decision::Blocked,
            ..base
        };
        assert_eq!(blocked.exit_code(), 1);
    }
}
