// SPDX-License-Identifier: AGPL-3.0-only
//! Grants: what a principal has let an agent do in its name, and no more.
//!
//! An agent has no identity of its own; it acts under a grant (ADR 0020 §2). A
//! grant is issued by a principal to an agent `(implementation, model)`, scoped
//! to repositories, capped by a tier ceiling, limited to a set of acts, bounded
//! in time, and revocable at any moment.
//!
//! Three properties are enforced here rather than hoped for:
//!
//! - **What a grant can confer scales with what the act claims** (ADR 0020 §4).
//!   [`Act`] has a variant for each delegable act and *no variant* for the
//!   non-delegable ones — an approval attestation, an authorship claim that
//!   asserts a legal position, any change to grants. A grant cannot name what
//!   the type cannot spell, so the ingest boundary that later checks a request
//!   against a grant cannot be talked into an attestation by any encoding.
//! - **A ceiling is a cap and never a promotion.** [`Grant::hop`] turns a grant
//!   into a [`Delegation`] for the evaluator, which takes the minimum along the
//!   chain (`Actor::effective_tier`, conformance clause 11). Nothing here
//!   reaches `Actor::tier` upward, and the property test beside this module
//!   says so over every tier and ceiling.
//! - **The clock stays outside.** Every method that depends on time takes `now`
//!   from the caller. The service supplies it; the tests choose it; nothing in
//!   this crate reads it.
//!
//! The credential that carries a grant to a resource server (ADR 0020 §5) is a
//! later slice. This module is the record the credential will be minted from
//! and checked against.

use crate::registry::{Registry, RegistryError};
use gitlocus_core::{Delegation, TrustTier};
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeSet;
use std::fmt;

/// An act a grant may confer. Only the delegable ones exist.
///
/// ADR 0020 §4 lists what a grant may and may not confer. The rows marked
/// non-delegable have no variant here on purpose: an approval attestation, an
/// authorship claim of `human`, `directed_agent` or `derived`, and issuing or
/// changing grants all require the principal's own credential, and a type that
/// cannot spell them is a stronger guarantee than a check that refuses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Act {
    /// Propose a contribution: that work exists.
    Propose,
    /// Emit `deterministic` or `assessed` evidence: what ran, or an opinion.
    Evidence,
    /// Declare authorship `generated`: no creative control is claimed, so
    /// nothing legal is asserted.
    DeclareGenerated,
}

impl Act {
    /// Every act a grant may confer.
    pub const ALL: [Self; 3] = [Self::Propose, Self::Evidence, Self::DeclareGenerated];

    /// The name a grant carries for this act, as written and as stored.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Propose => "propose",
            Self::Evidence => "evidence",
            Self::DeclareGenerated => "declare_generated",
        }
    }

    /// The act named, if a grant may confer it. `None` for anything else,
    /// including every non-delegable act: `attest`, `approve`,
    /// `declare_human` and the rest have no spelling here.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|act| act.name() == name)
    }
}

impl fmt::Display for Act {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The agent a grant is issued to: the two machine halves of ADR 0007's triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    /// The harness, e.g. `claude-code`.
    pub implementation: String,
    /// The model, when the harness reports one.
    pub model: Option<String>,
}

/// What a principal asks to grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRequest {
    /// The principal issuing it; must exist.
    pub principal: String,
    /// The agent it is issued to.
    pub agent: AgentIdentity,
    /// Canonical repository identifiers the grant covers. Never empty.
    pub repositories: Vec<String>,
    /// The most standing the agent may hold under it. A cap, never a promotion.
    pub ceiling: TrustTier,
    /// What the agent may do. Never empty.
    pub acts: BTreeSet<Act>,
    /// When it is issued, seconds since the Unix epoch, supplied by the caller.
    pub issued_at: i64,
    /// When it stops working, seconds since the Unix epoch. After `issued_at`.
    pub expires_at: i64,
}

/// A grant as recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// Stable identifier, minted here.
    pub id: String,
    /// The principal that issued it and is answerable for what it permits.
    pub principal: String,
    /// The agent it was issued to.
    pub agent: AgentIdentity,
    /// The repositories it covers, sorted.
    pub repositories: Vec<String>,
    /// Its ceiling.
    pub ceiling: TrustTier,
    /// The acts it confers.
    pub acts: BTreeSet<Act>,
    /// When it was issued.
    pub issued_at: i64,
    /// When it expires.
    pub expires_at: i64,
    /// When it was revoked, if it has been. Revocation is permanent.
    pub revoked_at: Option<i64>,
}

impl Grant {
    /// The hop this grant adds to an actor's delegation chain.
    ///
    /// This is the only way a grant reaches the evaluator, and it carries the
    /// ceiling as a ceiling: `Actor::effective_tier` takes the minimum of the
    /// root's tier and every hop, so a grant can lower standing and never raise
    /// it. That property is tested beside this module over every pair of tiers.
    #[must_use]
    pub fn hop(&self) -> Delegation {
        Delegation {
            delegator: self.principal.clone(),
            ceiling: self.ceiling,
            grant: Some(self.id.clone()),
        }
    }

    /// Whether the grant is live at `now`: issued, not expired, not revoked.
    #[must_use]
    pub fn is_live_at(&self, now: i64) -> bool {
        self.revoked_at.is_none() && now >= self.issued_at && now < self.expires_at
    }
}

/// What went wrong issuing, reading or revoking a grant.
#[derive(Debug)]
pub enum GrantError {
    /// The issuing principal does not exist.
    UnknownPrincipal(String),
    /// A grant covering no repository would permit nothing anywhere.
    EmptyScope,
    /// A grant conferring no act would permit nothing at all.
    NoActs,
    /// The grant would be dead on arrival.
    ExpiresBeforeIssue {
        /// When it would be issued.
        issued_at: i64,
        /// When it would expire.
        expires_at: i64,
    },
    /// No grant has this identifier.
    UnknownGrant(String),
    /// The grant was already revoked; revocation is permanent and once.
    AlreadyRevoked {
        /// The grant.
        grant: String,
        /// When it was revoked.
        at: i64,
    },
    /// The database itself failed.
    Storage(rusqlite::Error),
}

impl fmt::Display for GrantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPrincipal(id) => write!(f, "no principal {id} to issue a grant"),
            Self::EmptyScope => f.write_str("a grant must name at least one repository"),
            Self::NoActs => f.write_str("a grant must confer at least one act"),
            Self::ExpiresBeforeIssue {
                issued_at,
                expires_at,
            } => write!(
                f,
                "a grant issued at {issued_at} cannot expire at {expires_at}"
            ),
            Self::UnknownGrant(id) => write!(f, "no grant {id}"),
            Self::AlreadyRevoked { grant, at } => {
                write!(f, "grant {grant} was already revoked at {at}")
            }
            Self::Storage(e) => write!(f, "storage: {e}"),
        }
    }
}

impl std::error::Error for GrantError {}

impl From<rusqlite::Error> for GrantError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e)
    }
}

impl From<RegistryError> for GrantError {
    fn from(e: RegistryError) -> Self {
        match e {
            RegistryError::Storage(inner) => Self::Storage(inner),
            RegistryError::UnknownPrincipal(id) => Self::UnknownPrincipal(id),
            // The registry's other errors are about bindings and cannot arise
            // from the calls this module makes; naming them here would claim
            // a path that does not exist.
            other => Self::Storage(rusqlite::Error::InvalidParameterName(other.to_string())),
        }
    }
}

/// Why a request under a grant is refused. Each names the fact that decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No such grant.
    UnknownGrant(String),
    /// The grant was revoked, and revocation is immediate and permanent.
    Revoked {
        /// When.
        at: i64,
    },
    /// The grant has expired, however valid it was.
    Expired {
        /// When it expired.
        at: i64,
    },
    /// The grant is not yet in force.
    NotYetIssued {
        /// When it will be.
        at: i64,
    },
    /// The grant does not cover this repository.
    OutOfScope {
        /// The repository asked about.
        repository: String,
    },
    /// The grant does not confer this act.
    ActNotGranted {
        /// The act asked about.
        act: Act,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownGrant(id) => write!(f, "no grant {id}"),
            Self::Revoked { at } => write!(f, "the grant was revoked at {at}"),
            Self::Expired { at } => write!(f, "the grant expired at {at}"),
            Self::NotYetIssued { at } => write!(f, "the grant is not in force until {at}"),
            Self::OutOfScope { repository } => {
                write!(f, "the grant does not cover {repository}")
            }
            Self::ActNotGranted { act } => write!(f, "the grant does not confer {act}"),
        }
    }
}

/// A request a live grant permits: who is answerable, which agent, and the hop
/// the evaluator will attenuate through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorised {
    /// The principal at the root of the chain.
    pub principal: String,
    /// The agent acting.
    pub agent: AgentIdentity,
    /// The hop to append to the actor's delegation chain.
    pub hop: Delegation,
}

fn tier_name(tier: TrustTier) -> &'static str {
    match tier {
        TrustTier::Unknown => "unknown",
        TrustTier::Vouched => "vouched",
        TrustTier::Contributor => "contributor",
        TrustTier::Maintainer => "maintainer",
    }
}

fn tier_from(name: &str) -> rusqlite::Result<TrustTier> {
    match name {
        "unknown" => Ok(TrustTier::Unknown),
        "vouched" => Ok(TrustTier::Vouched),
        "contributor" => Ok(TrustTier::Contributor),
        "maintainer" => Ok(TrustTier::Maintainer),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "stored ceiling {other} is not a tier"
        ))),
    }
}

impl Registry {
    /// Issue a grant.
    ///
    /// # Errors
    /// [`GrantError::UnknownPrincipal`], [`GrantError::EmptyScope`],
    /// [`GrantError::NoActs`] and [`GrantError::ExpiresBeforeIssue`] as their
    /// names say; nothing is written when any of them applies.
    pub fn issue(&self, request: GrantRequest) -> Result<Grant, GrantError> {
        if request.repositories.is_empty() {
            return Err(GrantError::EmptyScope);
        }
        if request.acts.is_empty() {
            return Err(GrantError::NoActs);
        }
        if request.expires_at <= request.issued_at {
            return Err(GrantError::ExpiresBeforeIssue {
                issued_at: request.issued_at,
                expires_at: request.expires_at,
            });
        }
        let tx = self.conn.unchecked_transaction()?;
        if self.get(&request.principal)?.is_none() {
            return Err(GrantError::UnknownPrincipal(request.principal));
        }
        let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute(
            "INSERT INTO grants (id, principal, implementation, model, ceiling, issued_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                request.principal,
                request.agent.implementation,
                request.agent.model,
                tier_name(request.ceiling),
                request.issued_at,
                request.expires_at,
            ],
        )?;
        let mut repositories: Vec<String> = request.repositories;
        repositories.sort();
        repositories.dedup();
        for repository in &repositories {
            tx.execute(
                "INSERT INTO grant_repositories (grant_id, repository) VALUES (?1, ?2)",
                params![id, repository],
            )?;
        }
        for act in &request.acts {
            tx.execute(
                "INSERT INTO grant_acts (grant_id, act) VALUES (?1, ?2)",
                params![id, act.name()],
            )?;
        }
        tx.commit()?;
        Ok(Grant {
            id,
            principal: request.principal,
            agent: request.agent,
            repositories,
            ceiling: request.ceiling,
            acts: request.acts,
            issued_at: request.issued_at,
            expires_at: request.expires_at,
            revoked_at: None,
        })
    }

    /// The grant with this identifier, if any.
    ///
    /// # Errors
    /// Fails only on a storage error.
    pub fn grant(&self, id: &str) -> Result<Option<Grant>, GrantError> {
        let head = self
            .conn
            .query_row(
                "SELECT principal, implementation, model, ceiling, issued_at, expires_at, revoked_at
                 FROM grants WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                    ))
                },
            )
            .optional()?;
        let Some((principal, implementation, model, ceiling, issued_at, expires_at, revoked_at)) =
            head
        else {
            return Ok(None);
        };
        let mut stmt = self.conn.prepare(
            "SELECT repository FROM grant_repositories WHERE grant_id = ?1 ORDER BY repository",
        )?;
        let repositories = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = self
            .conn
            .prepare("SELECT act FROM grant_acts WHERE grant_id = ?1")?;
        let acts = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|name| {
                Act::parse(&name).ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "stored act {name} is not one a grant can confer"
                    ))
                })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok(Some(Grant {
            id: id.to_string(),
            principal,
            agent: AgentIdentity {
                implementation,
                model,
            },
            repositories,
            ceiling: tier_from(&ceiling)?,
            acts,
            issued_at,
            expires_at,
            revoked_at,
        }))
    }

    /// Every grant a principal has issued, newest first. Revoked and expired
    /// grants are included: the Grants screen shows the whole record, and a
    /// revocation somebody cannot see is a revocation they cannot trust.
    ///
    /// # Errors
    /// Fails only on a storage error.
    pub fn grants_of(&self, principal: &str) -> Result<Vec<Grant>, GrantError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM grants WHERE principal = ?1 ORDER BY issued_at DESC, id")?;
        let ids = stmt
            .query_map(params![principal], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut grants = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(grant) = self.grant(&id)? {
                grants.push(grant);
            }
        }
        Ok(grants)
    }

    /// Revoke a grant at `at`. Immediate, permanent, and once.
    ///
    /// # Errors
    /// [`GrantError::UnknownGrant`]; [`GrantError::AlreadyRevoked`] when it
    /// already was, so a second revocation cannot quietly move the timestamp.
    pub fn revoke(&self, id: &str, at: i64) -> Result<Grant, GrantError> {
        let tx = self.conn.unchecked_transaction()?;
        let Some(existing) = self.grant(id)? else {
            return Err(GrantError::UnknownGrant(id.to_string()));
        };
        if let Some(when) = existing.revoked_at {
            return Err(GrantError::AlreadyRevoked {
                grant: id.to_string(),
                at: when,
            });
        }
        tx.execute(
            "UPDATE grants SET revoked_at = ?2 WHERE id = ?1",
            params![id, at],
        )?;
        tx.commit()?;
        Ok(Grant {
            revoked_at: Some(at),
            ..existing
        })
    }

    /// Whether a grant permits `act` in `repository` at `now`, and if so, whose
    /// authority that is.
    ///
    /// The outer `Result` is storage; the inner one is the decision. A refusal
    /// is an answer, not an error, and it names the fact that decided it.
    ///
    /// # Errors
    /// Fails only on a storage error.
    pub fn authorise(
        &self,
        grant_id: &str,
        now: i64,
        repository: &str,
        act: Act,
    ) -> Result<Result<Authorised, Refusal>, GrantError> {
        let Some(grant) = self.grant(grant_id)? else {
            return Ok(Err(Refusal::UnknownGrant(grant_id.to_string())));
        };
        // Revocation is checked first: a revoked grant is refused whatever else
        // is true of it, and the reason a caller sees should be the one that
        // will not change if they wait.
        if let Some(at) = grant.revoked_at {
            return Ok(Err(Refusal::Revoked { at }));
        }
        if now >= grant.expires_at {
            return Ok(Err(Refusal::Expired {
                at: grant.expires_at,
            }));
        }
        if now < grant.issued_at {
            return Ok(Err(Refusal::NotYetIssued {
                at: grant.issued_at,
            }));
        }
        if !grant.repositories.iter().any(|r| r == repository) {
            return Ok(Err(Refusal::OutOfScope {
                repository: repository.to_string(),
            }));
        }
        if !grant.acts.contains(&act) {
            return Ok(Err(Refusal::ActNotGranted { act }));
        }
        Ok(Ok(Authorised {
            principal: grant.principal.clone(),
            agent: grant.agent.clone(),
            hop: grant.hop(),
        }))
    }
}
