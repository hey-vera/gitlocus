// SPDX-License-Identifier: Apache-2.0
//! What a named party declares about how a change was produced.
//!
//! Not a sixth type. An authorship claim rides on an [`crate::evidence::Evidence`]
//! record of class `attested`, because that is exactly what it is: a human
//! accepting responsibility for a statement.
//!
//! **This is a declaration, not a detection**, and the distinction is the whole
//! point — see [ADR 0008](https://github.com/hey-vera/gitlocus/blob/main/docs/adr/0008-authorship-is-declared-not-detected.md).
//! Nothing here inspects source text and guesses. A named party states something
//! and signs it, and if the statement is false they have signed a false
//! statement. That is the same instrument as the DCO, which works for exactly
//! that reason.
//!
//! Why it matters: the US Copyright Office holds that AI output needs sufficient
//! human control over the expressive elements to be copyrightable, and that
//! prompts alone do not supply it. Code failing that bar is effectively public
//! domain, and public domain code inside a copyleft project is not bound by the
//! copyleft. So copyleft does not die by being violated; it dies by dilution,
//! quietly, with nothing visibly breaking. Recording the claim at the moment of
//! production is what makes that refusable as policy.

use serde::{Deserialize, Serialize};
use std::fmt;

/// What a party declares about how a change was produced.
///
/// The variants map onto the USCO framework rather than onto any taxonomy of
/// tools: what matters is whether a human exercised creative control over the
/// expressive choices, not which editor or model was involved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "claim", rename_all = "snake_case")]
pub enum AuthorshipClaim {
    /// A human wrote it. Copyrightable by the declarer.
    Human,
    /// An agent produced it; a human directed and revised the expressive choices
    /// and asserts sufficient creative control over them.
    DirectedAgent,
    /// An agent produced it and no human claims creative control over the
    /// expressive elements. Likely uncopyrightable, and therefore likely not
    /// bound by the repository's licence.
    ///
    /// This is the variant that does the work. It makes the previously invisible
    /// thing refusable, and it is what a contribution is taken to be when it
    /// declares nothing at all.
    Generated,
    /// Copied or adapted from an identified external source.
    Derived {
        /// Where it came from. Free text; a URL is better than a name.
        source: String,
        /// The licence it arrived under, where one is known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        license: Option<String>,
    },
}

impl AuthorshipClaim {
    /// The claim's discriminant, which is what a policy names.
    #[must_use]
    pub fn kind(&self) -> AuthorshipKind {
        match self {
            Self::Human => AuthorshipKind::Human,
            Self::DirectedAgent => AuthorshipKind::DirectedAgent,
            Self::Generated => AuthorshipKind::Generated,
            Self::Derived { .. } => AuthorshipKind::Derived,
        }
    }
}

/// An authorship claim with its detail stripped, as a policy names it.
///
/// A policy says `authorship: [human, directed_agent]`. It cannot usefully name
/// the source of a `derived` claim, so the discriminant is the unit of policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorshipKind {
    /// See [`AuthorshipClaim::Human`].
    Human,
    /// See [`AuthorshipClaim::DirectedAgent`].
    DirectedAgent,
    /// See [`AuthorshipClaim::Generated`].
    Generated,
    /// See [`AuthorshipClaim::Derived`].
    Derived,
}

impl fmt::Display for AuthorshipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Human => "human",
            Self::DirectedAgent => "directed_agent",
            Self::Generated => "generated",
            Self::Derived => "derived",
        };
        f.write_str(name)
    }
}

/// What a contribution is taken to have declared when it declares nothing.
///
/// **Silence is not a claim.** Absent a declaration, the weakest one applies, so
/// nobody acquires ownership by saying nothing and asserting authorship stays a
/// deliberate act. A default of `human` would let every unlabelled contribution
/// quietly claim copyright on the project's behalf, which is the failure this
/// whole mechanism exists to prevent.
pub const UNDECLARED: AuthorshipKind = AuthorshipKind::Generated;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_taken_as_generated_not_as_human() {
        // If this ever defaults the other way, every unlabelled contribution
        // silently claims copyright and the mechanism inverts.
        assert_eq!(UNDECLARED, AuthorshipKind::Generated);
    }

    #[test]
    fn a_claim_reports_the_kind_a_policy_names() {
        assert_eq!(AuthorshipClaim::Human.kind(), AuthorshipKind::Human);
        assert_eq!(
            AuthorshipClaim::DirectedAgent.kind(),
            AuthorshipKind::DirectedAgent
        );
        assert_eq!(AuthorshipClaim::Generated.kind(), AuthorshipKind::Generated);
        assert_eq!(
            AuthorshipClaim::Derived {
                source: "https://example.com/thing".into(),
                license: Some("MIT".into()),
            }
            .kind(),
            AuthorshipKind::Derived
        );
    }

    #[test]
    fn a_claim_round_trips_with_its_detail_intact() {
        // The source and licence of a derived claim are the whole value of it to
        // an auditor; losing them in serialisation would leave a record saying
        // only that something was copied from somewhere.
        let claim = AuthorshipClaim::Derived {
            source: "https://github.com/acme/widgets".into(),
            license: Some("Apache-2.0".into()),
        };
        let json = serde_json::to_string(&claim).unwrap();
        assert!(json.contains(r#""claim":"derived""#), "{json}");
        assert!(json.contains("acme/widgets"), "{json}");
        assert_eq!(
            serde_json::from_str::<AuthorshipClaim>(&json).unwrap(),
            claim
        );
    }

    #[test]
    fn a_unit_claim_serialises_as_a_tagged_object() {
        // One shape for every variant, so a schema can describe the field and a
        // consumer does not have to handle a bare string and an object.
        let json = serde_json::to_string(&AuthorshipClaim::Human).unwrap();
        assert_eq!(json, r#"{"claim":"human"}"#);
    }

    #[test]
    fn a_kind_names_itself_as_a_policy_writes_it() {
        assert_eq!(AuthorshipKind::DirectedAgent.to_string(), "directed_agent");
    }
}
