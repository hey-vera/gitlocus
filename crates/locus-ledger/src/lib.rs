// SPDX-License-Identifier: AGPL-3.0-only
//! `locus-ledger` — the stateful half of GitLocus, kept beside the evaluator.
//!
//! [`locusd`](../locusd) evaluates and stores nothing. This crate is where
//! everything with a lifetime lives: who a principal is and which upstream
//! logins are theirs ([`registry`]), and — as later slices land — the grants a
//! principal has issued to agents, the credentials that carry them, and the
//! record of what was decided.
//!
//! The one rule that shapes every module here is
//! [ADR 0021](../../docs/adr/0021-state-lives-beside-the-evaluator-never-in-it.md):
//! this crate may call `gitlocus_core::evaluate`; nothing in the evaluator may
//! reach back into this crate's state. The dependency graph is the proof.

pub mod registry;

pub use registry::{Binding, Principal, Registry, RegistryError, SCHEMA_VERSION};
