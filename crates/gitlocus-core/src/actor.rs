// SPDX-License-Identifier: Apache-2.0
//! Who produced a change.

use serde::{Deserialize, Serialize};

/// What kind of thing produced a contribution.
///
/// [`ActorKind::Pair`] is the case GitLocus exists to represent. An autonomous
/// agent acting on a human's instruction is neither purely the human nor purely
/// the agent: the agent did the work and the human is answerable for it. Systems
/// that force this into a single identity lose whichever half they discard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ActorKind {
    /// A person acting directly.
    Human,
    /// An autonomous agent with no identified operator. Deliberately weak: an
    /// agent nobody will answer for should not clear a meaningful trust tier.
    Agent {
        /// Stable identifier for the agent implementation, e.g. `claude-code`.
        implementation: String,
    },
    /// An agent acting under an identified operator who accepts responsibility.
    Pair {
        /// Stable identifier for the agent implementation.
        implementation: String,
        /// Identity of the human who dispatched the agent and is answerable.
        operator: String,
    },
}

/// How much standing an actor has in a given repository.
///
/// Ordering is meaningful: a rule asking for [`TrustTier::Contributor`] is also
/// satisfied by a [`TrustTier::Maintainer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    /// No established standing. The default for anyone the repository has not seen.
    Unknown,
    /// Explicitly vouched for by someone the repository already trusts.
    ///
    /// Interoperates with `VOUCHED.td` as used by `mitchellh/vouch`; GitLocus
    /// reads that format rather than defining a competing one.
    Vouched,
    /// Has landed changes in this repository before.
    Contributor,
    /// Carries review authority in this repository.
    Maintainer,
}

impl TrustTier {
    /// Whether this tier satisfies a requirement for `required`.
    #[must_use]
    pub fn satisfies(self, required: Self) -> bool {
        self >= required
    }
}

/// An identity that can produce contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// Stable identifier, unique within the issuing identity provider.
    pub id: String,
    /// What kind of thing this actor is.
    #[serde(flatten)]
    pub kind: ActorKind,
    /// Trust tier for the repository being evaluated.
    #[serde(default = "default_tier")]
    pub tier: TrustTier,
    /// Identity of the key or OIDC subject the actor's signatures verify against.
    ///
    /// `None` means unauthenticated: claims made by this actor are self-asserted
    /// and carry no cryptographic weight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_binding: Option<String>,
}

fn default_tier() -> TrustTier {
    TrustTier::Unknown
}

impl Actor {
    /// Whether the actor names a human who is answerable for the work.
    ///
    /// A bare [`ActorKind::Agent`] does not: nobody has accepted responsibility.
    #[must_use]
    pub fn responsible_human(&self) -> Option<&str> {
        match &self.kind {
            ActorKind::Human => Some(&self.id),
            ActorKind::Pair { operator, .. } => Some(operator),
            ActorKind::Agent { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_are_ordered() {
        assert!(TrustTier::Maintainer.satisfies(TrustTier::Contributor));
        assert!(!TrustTier::Unknown.satisfies(TrustTier::Vouched));
        assert!(TrustTier::Vouched.satisfies(TrustTier::Vouched));
    }

    #[test]
    fn unattended_agents_have_no_responsible_human() {
        let agent = Actor {
            id: "agent-1".into(),
            kind: ActorKind::Agent {
                implementation: "claude-code".into(),
            },
            tier: TrustTier::Unknown,
            key_binding: None,
        };
        assert_eq!(agent.responsible_human(), None);

        let pair = Actor {
            id: "agent-1".into(),
            kind: ActorKind::Pair {
                implementation: "claude-code".into(),
                operator: "josh".into(),
            },
            tier: TrustTier::Contributor,
            key_binding: Some("https://github.com/login/oauth".into()),
        };
        assert_eq!(pair.responsible_human(), Some("josh"));
    }
}
