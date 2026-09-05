// SPDX-License-Identifier: AGPL-3.0-only
//! Principals, and the upstream identities bound to them.
//!
//! A principal is GitLocus's durable identity for an answerable party — the
//! `operator` in ADR 0007's triple. It is durable because it is *not* any one
//! login: it binds one or more upstream identities (a GitHub account first;
//! GitLab and passkeys later), and it survives losing any one of them as long
//! as one remains. Standing attaches to the principal, which is what lets it
//! survive leaving a host (ADR 0020 §1).
//!
//! Two invariants, both enforced here and tested beside this module:
//!
//! - **An upstream identity belongs to at most one principal.** Otherwise one
//!   login could carry two standings, and the Sybil argument in ADR 0007 fails.
//! - **A principal always holds at least one binding.** Unbinding the last one
//!   is refused. A principal nobody can log in as is a principal nobody can be
//!   answerable through, and there is no recovery flow to reach it (ADR 0020:
//!   no passwords, no reset).
//!
//! GitLocus issues no login credential of its own. What arrives here is an
//! identity an upstream already authenticated; this module only records which
//! principal it is.

use rusqlite::{Connection, OptionalExtension, params};
use std::fmt;
use std::path::Path;

/// The shape of the database this build writes and reads.
///
/// Migrations are forward-only. A database at a newer version than this is
/// refused rather than read, because the store is the one asset that cannot be
/// reconstructed and a silent misread of it is the worst available failure.
pub const SCHEMA_VERSION: u32 = 1;

/// One upstream identity: who an identity provider says this is.
///
/// `provider` and `subject` together are the stable key — a GitHub account
/// keeps its numeric `id` through any number of renames — and `login` is the
/// human-readable name at the time of binding, kept for display only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The identity provider, e.g. `github`.
    pub provider: String,
    /// The provider's stable identifier for this identity.
    pub subject: String,
    /// The provider's human-readable name for it, as last seen.
    pub login: String,
}

/// An answerable party, as GitLocus knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// Stable identifier, minted here and never derived from any upstream.
    pub id: String,
    /// Every upstream identity that resolves to this principal. Never empty.
    pub bindings: Vec<Binding>,
}

/// What can go wrong in the registry.
#[derive(Debug)]
pub enum RegistryError {
    /// The database was written by a newer build than this one.
    SchemaFromTheFuture {
        /// The version the database declares.
        found: u32,
    },
    /// Removing this binding would leave the principal with none.
    LastBinding {
        /// The principal that would be stranded.
        principal: String,
    },
    /// The upstream identity already resolves to a principal.
    AlreadyBound {
        /// The provider of the identity.
        provider: String,
        /// Its subject.
        subject: String,
        /// The principal it already belongs to.
        principal: String,
    },
    /// No principal has this identifier.
    UnknownPrincipal(String),
    /// The principal exists, but this identity is not one of its bindings.
    NotBound {
        /// The principal asked about.
        principal: String,
        /// The provider of the identity.
        provider: String,
        /// Its subject.
        subject: String,
    },
    /// The database itself failed.
    Storage(rusqlite::Error),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaFromTheFuture { found } => write!(
                f,
                "the database is at schema version {found}; this build reads up to {SCHEMA_VERSION}"
            ),
            Self::LastBinding { principal } => write!(
                f,
                "principal {principal} has one binding left and it cannot be removed"
            ),
            Self::AlreadyBound {
                provider,
                subject,
                principal,
            } => write!(
                f,
                "{provider}:{subject} is already bound to principal {principal}"
            ),
            Self::UnknownPrincipal(id) => write!(f, "no principal {id}"),
            Self::NotBound {
                principal,
                provider,
                subject,
            } => write!(
                f,
                "{provider}:{subject} is not a binding of principal {principal}"
            ),
            Self::Storage(e) => write!(f, "storage: {e}"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<rusqlite::Error> for RegistryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e)
    }
}

/// The principal registry, over one SQLite database.
#[derive(Debug)]
pub struct Registry {
    conn: Connection,
}

impl Registry {
    /// Open the database at `path`, creating it if absent, and bring it to the
    /// current schema.
    ///
    /// # Errors
    /// Fails if the file cannot be opened, if a migration fails, or if the
    /// database was written by a newer schema than this build knows.
    pub fn open(path: &Path) -> Result<Self, RegistryError> {
        Self::from_connection(Connection::open(path)?)
    }

    /// A registry that lives only as long as this value. For tests.
    ///
    /// # Errors
    /// Fails only if SQLite cannot allocate the database.
    pub fn in_memory() -> Result<Self, RegistryError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, RegistryError> {
        // Enforced, not assumed: a binding row pointing at a principal that is
        // gone would be exactly the dangling standing this module exists to
        // prevent.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let registry = Self { conn };
        registry.migrate()?;
        Ok(registry)
    }

    /// The schema version the database is at.
    ///
    /// # Errors
    /// Fails if the pragma cannot be read.
    pub fn schema_version(&self) -> Result<u32, RegistryError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// Forward-only migrations, one step per version, each in its own
    /// transaction so a failure leaves the database at a version that is
    /// exactly what it says it is.
    fn migrate(&self) -> Result<(), RegistryError> {
        let mut version = self.schema_version()?;
        if version > SCHEMA_VERSION {
            return Err(RegistryError::SchemaFromTheFuture { found: version });
        }
        while version < SCHEMA_VERSION {
            let next = version + 1;
            let tx = self.conn.unchecked_transaction()?;
            match next {
                1 => tx.execute_batch(
                    "CREATE TABLE principals (
                         id TEXT PRIMARY KEY,
                         created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                     );
                     CREATE TABLE bindings (
                         provider  TEXT NOT NULL,
                         subject   TEXT NOT NULL,
                         login     TEXT NOT NULL,
                         principal TEXT NOT NULL REFERENCES principals(id),
                         bound_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                         PRIMARY KEY (provider, subject)
                     );
                     CREATE INDEX bindings_by_principal ON bindings(principal);",
                )?,
                // The loop cannot reach a version this build does not define:
                // the check above refuses anything past SCHEMA_VERSION, and every
                // version up to it has an arm here.
                _ => unreachable!("no migration defined for schema version {next}"),
            }
            tx.pragma_update(None, "user_version", next)?;
            tx.commit()?;
            version = next;
        }
        Ok(())
    }

    /// The principal an upstream identity resolves to, if any.
    ///
    /// # Errors
    /// Fails only on a storage error.
    pub fn resolve(
        &self,
        provider: &str,
        subject: &str,
    ) -> Result<Option<Principal>, RegistryError> {
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT principal FROM bindings WHERE provider = ?1 AND subject = ?2",
                params![provider, subject],
                |row| row.get(0),
            )
            .optional()?;
        match id {
            Some(id) => self.get(&id),
            None => Ok(None),
        }
    }

    /// The principal with this identifier, if any.
    ///
    /// # Errors
    /// Fails only on a storage error.
    pub fn get(&self, id: &str) -> Result<Option<Principal>, RegistryError> {
        let exists: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM principals WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(id) = exists else {
            return Ok(None);
        };
        let mut stmt = self.conn.prepare(
            "SELECT provider, subject, login FROM bindings
             WHERE principal = ?1 ORDER BY bound_at, provider, subject",
        )?;
        let bindings = stmt
            .query_map(params![id], |row| {
                Ok(Binding {
                    provider: row.get(0)?,
                    subject: row.get(1)?,
                    login: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Principal { id, bindings }))
    }

    /// Create a principal for an upstream identity seen for the first time.
    ///
    /// The identifier is minted here from SQLite's random source and has no
    /// relationship to the upstream: a principal that outlives its first login
    /// must not be named after it.
    ///
    /// # Errors
    /// [`RegistryError::AlreadyBound`] if the identity already resolves to a
    /// principal — enrol is for strangers; a known identity logs in.
    pub fn enrol(&self, binding: Binding) -> Result<Principal, RegistryError> {
        let tx = self.conn.unchecked_transaction()?;
        Self::refuse_if_bound(&tx, &binding)?;
        let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?;
        tx.execute("INSERT INTO principals (id) VALUES (?1)", params![id])?;
        tx.execute(
            "INSERT INTO bindings (provider, subject, login, principal) VALUES (?1, ?2, ?3, ?4)",
            params![binding.provider, binding.subject, binding.login, id],
        )?;
        tx.commit()?;
        Ok(Principal {
            id,
            bindings: vec![binding],
        })
    }

    /// Bind a further upstream identity to an existing principal.
    ///
    /// # Errors
    /// [`RegistryError::UnknownPrincipal`] if there is no such principal;
    /// [`RegistryError::AlreadyBound`] if the identity belongs to one already,
    /// including this one.
    pub fn bind(&self, principal: &str, binding: &Binding) -> Result<Principal, RegistryError> {
        let tx = self.conn.unchecked_transaction()?;
        if !Self::exists(&tx, principal)? {
            return Err(RegistryError::UnknownPrincipal(principal.to_string()));
        }
        Self::refuse_if_bound(&tx, binding)?;
        tx.execute(
            "INSERT INTO bindings (provider, subject, login, principal) VALUES (?1, ?2, ?3, ?4)",
            params![binding.provider, binding.subject, binding.login, principal],
        )?;
        tx.commit()?;
        self.get(principal)?
            .ok_or_else(|| RegistryError::UnknownPrincipal(principal.to_string()))
    }

    /// Remove an upstream identity from a principal.
    ///
    /// # Errors
    /// [`RegistryError::LastBinding`] if it is the only one — a principal with
    /// no way to log in is unreachable and unanswerable, and there is no
    /// recovery flow to bring it back; [`RegistryError::UnknownPrincipal`] and
    /// [`RegistryError::NotBound`] as their names say.
    pub fn unbind(
        &self,
        principal: &str,
        provider: &str,
        subject: &str,
    ) -> Result<Principal, RegistryError> {
        let tx = self.conn.unchecked_transaction()?;
        if !Self::exists(&tx, principal)? {
            return Err(RegistryError::UnknownPrincipal(principal.to_string()));
        }
        let held: u32 = tx.query_row(
            "SELECT count(*) FROM bindings WHERE principal = ?1",
            params![principal],
            |row| row.get(0),
        )?;
        let is_bound: bool = tx.query_row(
            "SELECT count(*) FROM bindings WHERE principal = ?1 AND provider = ?2 AND subject = ?3",
            params![principal, provider, subject],
            |row| row.get::<_, u32>(0).map(|n| n == 1),
        )?;
        if !is_bound {
            return Err(RegistryError::NotBound {
                principal: principal.to_string(),
                provider: provider.to_string(),
                subject: subject.to_string(),
            });
        }
        if held <= 1 {
            return Err(RegistryError::LastBinding {
                principal: principal.to_string(),
            });
        }
        tx.execute(
            "DELETE FROM bindings WHERE principal = ?1 AND provider = ?2 AND subject = ?3",
            params![principal, provider, subject],
        )?;
        tx.commit()?;
        self.get(principal)?
            .ok_or_else(|| RegistryError::UnknownPrincipal(principal.to_string()))
    }

    fn exists(conn: &Connection, principal: &str) -> Result<bool, RegistryError> {
        Ok(conn
            .query_row(
                "SELECT 1 FROM principals WHERE id = ?1",
                params![principal],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    fn refuse_if_bound(conn: &Connection, binding: &Binding) -> Result<(), RegistryError> {
        let owner: Option<String> = conn
            .query_row(
                "SELECT principal FROM bindings WHERE provider = ?1 AND subject = ?2",
                params![binding.provider, binding.subject],
                |row| row.get(0),
            )
            .optional()?;
        match owner {
            Some(principal) => Err(RegistryError::AlreadyBound {
                provider: binding.provider.clone(),
                subject: binding.subject.clone(),
                principal,
            }),
            None => Ok(()),
        }
    }
}
