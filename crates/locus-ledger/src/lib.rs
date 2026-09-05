// SPDX-License-Identifier: AGPL-3.0-only
//! `locus-ledger` — the stateful half of GitLocus, kept beside the evaluator.
//!
//! [`locusd`](../locusd) evaluates and stores nothing. This crate is where
//! everything with a lifetime lives: who a principal is and which upstream
//! logins are theirs ([`registry`]), what a principal has let an agent do in
//! its name ([`grants`]), and — as later slices land — the credentials that
//! carry a grant and the record of what was decided.
//!
//! The one rule that shapes every module here is
//! [ADR 0021](../../docs/adr/0021-state-lives-beside-the-evaluator-never-in-it.md):
//! this crate may call `gitlocus_core::evaluate`; nothing in the evaluator may
//! reach back into this crate's state. The dependency graph is the proof.

pub mod grants;
pub mod registry;

pub use grants::{Act, AgentIdentity, Authorised, Grant, GrantError, GrantRequest, Refusal};
pub use registry::{Binding, Principal, Registry, RegistryError, SCHEMA_VERSION};
