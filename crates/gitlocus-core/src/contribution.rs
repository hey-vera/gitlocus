// SPDX-License-Identifier: Apache-2.0
//! The proposed change.

use crate::actor::Actor;
use serde::{Deserialize, Serialize};

/// A proposed change to a repository.
///
/// Identified by content digests rather than by a forge's pull-request number, so
/// the same contribution keeps its identity across a GitHub PR, a GitLab merge
/// request, or a mailed patch series. `forge_ref` records where it was observed;
/// nothing in the model depends on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    /// Canonical repository identifier, e.g. `github.com/hey-vera/gitlocus`.
    pub repository: String,
    /// Digest of the revision this change is proposed against.
    pub base_digest: String,
    /// Digest of the proposed revision.
    pub head_digest: String,
    /// Who produced it.
    pub actor: Actor,
    /// Repository-relative paths this change touches.
    #[serde(default)]
    pub changed_paths: Vec<String>,
    /// Where this contribution was observed, if anywhere. Informational only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forge_ref: Option<String>,
}

impl Contribution {
    /// Stable identity of the change, independent of any forge.
    #[must_use]
    pub fn id(&self) -> String {
        format!(
            "{}@{}..{}",
            self.repository, self.base_digest, self.head_digest
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ActorKind, TrustTier};

    fn contribution(repository: &str, base: &str, head: &str) -> Contribution {
        Contribution {
            repository: repository.into(),
            base_digest: base.into(),
            head_digest: head.into(),
            actor: Actor {
                id: "someone".into(),
                kind: ActorKind::Human,
                tier: TrustTier::Unknown,
                key_binding: None,
            },
            changed_paths: Vec::new(),
            forge_ref: None,
        }
    }

    #[test]
    fn identity_is_the_repository_and_both_digests() {
        assert_eq!(
            contribution("github.com/hey-vera/gitlocus", "aaaa", "bbbb").id(),
            "github.com/hey-vera/gitlocus@aaaa..bbbb"
        );
    }

    #[test]
    fn identity_distinguishes_contributions_that_differ_in_any_component() {
        // The identity appears in every verdict, so two different changes
        // sharing one is not a cosmetic problem: it is two verdicts that cannot
        // be told apart by the thing naming them.
        let base = contribution("repo", "aaaa", "bbbb");
        for other in [
            contribution("other-repo", "aaaa", "bbbb"),
            contribution("repo", "cccc", "bbbb"),
            contribution("repo", "aaaa", "cccc"),
        ] {
            assert_ne!(base.id(), other.id());
        }
    }

    #[test]
    fn identity_ignores_where_it_was_observed() {
        // A GitHub pull request, a GitLab merge request and a mailed patch
        // series describing the same change are one contribution.
        let mut observed = contribution("repo", "aaaa", "bbbb");
        observed.forge_ref = Some("https://github.com/hey-vera/gitlocus/pull/1".into());
        assert_eq!(observed.id(), contribution("repo", "aaaa", "bbbb").id());
    }
}
