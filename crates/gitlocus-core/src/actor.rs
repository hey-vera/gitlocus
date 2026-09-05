// SPDX-License-Identifier: Apache-2.0
//! Who produced a change.

use serde::{Deserialize, Serialize};

/// What kind of thing produced a contribution.
///
/// [`ActorKind::Pair`] is the case GitLocus exists to represent. An autonomous
/// agent acting on a human's instruction is neither purely the human nor purely
/// the agent: the agent did the work and the human is answerable for it. Systems
/// that force this into a single identity lose whichever half they discard.
///
/// Standing attaches to the durable triple `(implementation, model, operator)`,
/// never to an instance or a session — see ADR 0007. The model is part of the
/// triple because a harness that silently swaps to a weaker model would otherwise
/// inherit standing it did not earn, and the change would be invisible.
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
        /// The model that produced the work, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// An agent acting under an identified operator who accepts responsibility.
    Pair {
        /// Stable identifier for the agent implementation.
        implementation: String,
        /// The model that produced the work, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
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

/// One hop in a delegation chain: somebody let this actor act for them, and
/// capped what it may hold while doing so.
///
/// A ceiling is a cap and never a promotion. It can lower the standing an actor
/// arrives with; it cannot raise it. That is scope attenuation (ADR 0007), and
/// [`Actor::effective_tier`] is where it is enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    /// Who granted this hop: the principal at the root, or the delegated actor
    /// one hop above.
    pub delegator: String,
    /// The most standing this hop may hold, whatever its delegator holds.
    pub ceiling: TrustTier,
    /// Identifier of the grant that created this hop. Informational only, like
    /// `forge_ref`: a verifier must not decide anything from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant: Option<String>,
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
    ///
    /// For a delegated actor this is the tier held at the *root* of the chain —
    /// what the answerable human has earned here. What the actor may actually
    /// exercise is [`Actor::effective_tier`], which is never higher.
    #[serde(default = "default_tier")]
    pub tier: TrustTier,
    /// Identity of the key or OIDC subject the actor's signatures verify against.
    ///
    /// `None` means unauthenticated: claims made by this actor are self-asserted
    /// and carry no cryptographic weight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_binding: Option<String>,
    /// The delegation chain that led to this actor, root first. Empty for an
    /// actor acting on its own standing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegation: Vec<Delegation>,
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

    /// The standing this actor may actually exercise: the tier at the root of
    /// its chain, lowered by every ceiling along it.
    ///
    /// Two rules from ADR 0007, both enforced here and nowhere else, so that a
    /// service constructing the chain cannot make the evaluator believe a higher
    /// tier than the chain permits:
    ///
    /// - a delegated actor never holds a tier above the actor that delegated
    ///   to it, so each hop can only lower the result;
    /// - a chain with no human at its root terminates at [`TrustTier::Unknown`],
    ///   whatever any hop asserts — attenuation from an unaccountable root
    ///   attenuates from nothing.
    #[must_use]
    pub fn effective_tier(&self) -> TrustTier {
        if self.responsible_human().is_none() {
            return TrustTier::Unknown;
        }
        self.delegation
            .iter()
            .fold(self.tier, |held, hop| held.min(hop.ceiling))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(tier: TrustTier, ceilings: &[TrustTier]) -> Actor {
        Actor {
            id: "josh".into(),
            kind: ActorKind::Pair {
                implementation: "claude-code".into(),
                model: Some("claude-fable-5-1".into()),
                operator: "josh".into(),
            },
            tier,
            key_binding: None,
            delegation: ceilings
                .iter()
                .map(|c| Delegation {
                    delegator: "josh".into(),
                    ceiling: *c,
                    grant: None,
                })
                .collect(),
        }
    }

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
                model: None,
            },
            tier: TrustTier::Unknown,
            key_binding: None,
            delegation: Vec::new(),
        };
        assert_eq!(agent.responsible_human(), None);

        let pair = pair(TrustTier::Contributor, &[]);
        assert_eq!(pair.responsible_human(), Some("josh"));
    }

    #[test]
    fn a_delegated_actor_never_holds_a_tier_above_its_delegator() {
        // A maintainer hands an agent a grant capped at contributor: the agent
        // acts as a contributor. The same grant from a contributor yields a
        // contributor too — the cap did nothing, and did not need to.
        assert_eq!(
            pair(TrustTier::Maintainer, &[TrustTier::Contributor]).effective_tier(),
            TrustTier::Contributor
        );
        assert_eq!(
            pair(TrustTier::Contributor, &[TrustTier::Contributor]).effective_tier(),
            TrustTier::Contributor
        );
        // A ceiling above the root's standing is not a promotion.
        assert_eq!(
            pair(TrustTier::Vouched, &[TrustTier::Maintainer]).effective_tier(),
            TrustTier::Vouched
        );
        // Two hops: the lowest ceiling anywhere in the chain wins.
        assert_eq!(
            pair(
                TrustTier::Maintainer,
                &[
                    TrustTier::Maintainer,
                    TrustTier::Vouched,
                    TrustTier::Contributor
                ]
            )
            .effective_tier(),
            TrustTier::Vouched
        );
    }

    #[test]
    fn a_chain_with_no_human_at_its_root_holds_no_standing() {
        // Whatever tier the document asserts, nobody is answerable for it.
        let agent = Actor {
            id: "agent-1".into(),
            kind: ActorKind::Agent {
                implementation: "claude-code".into(),
                model: Some("claude-fable-5-1".into()),
            },
            tier: TrustTier::Maintainer,
            key_binding: Some("https://example.com".into()),
            delegation: vec![Delegation {
                delegator: "another-agent".into(),
                ceiling: TrustTier::Maintainer,
                grant: Some("g-1".into()),
            }],
        };
        assert_eq!(agent.effective_tier(), TrustTier::Unknown);
    }

    #[test]
    fn an_undelegated_actor_keeps_the_tier_it_was_given() {
        for tier in [
            TrustTier::Unknown,
            TrustTier::Vouched,
            TrustTier::Contributor,
            TrustTier::Maintainer,
        ] {
            assert_eq!(pair(tier, &[]).effective_tier(), tier);
            let human = Actor {
                id: "someone".into(),
                kind: ActorKind::Human,
                tier,
                key_binding: None,
                delegation: Vec::new(),
            };
            assert_eq!(human.effective_tier(), tier);
        }
    }

    #[test]
    fn the_model_is_part_of_the_triple_and_round_trips() {
        let actor = pair(TrustTier::Contributor, &[TrustTier::Contributor]);
        let json = serde_json::to_string(&actor).expect("serialise");
        assert!(json.contains("\"model\":\"claude-fable-5-1\""));
        assert!(json.contains("\"delegation\""));
        let back: Actor = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, actor);
    }

    #[test]
    fn a_document_written_before_delegation_existed_still_parses() {
        // The fields are additive. A contribution recorded under the old shape
        // must evaluate exactly as it did: no chain, tier as given.
        let json = r#"{"id":"someone","kind":"pair","implementation":"claude-code","operator":"josh","tier":"contributor"}"#;
        let actor: Actor = serde_json::from_str(json).expect("old shape parses");
        assert!(actor.delegation.is_empty());
        assert_eq!(actor.effective_tier(), TrustTier::Contributor);
        // And nothing new is written out for it, so a round trip is byte-stable.
        let again = serde_json::to_string(&actor).expect("serialise");
        assert!(!again.contains("delegation"));
        assert!(!again.contains("model"));
    }
}
