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
