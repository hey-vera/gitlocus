// SPDX-License-Identifier: Apache-2.0
//! Core model for GitLocus.
//!
//! GitLocus answers one question: *given who produced this change and what can be
//! proven about it, what still has to happen before a human should look at it?*
//!
//! The model is deliberately small. Five types carry it:
//!
//! - [`actor::Actor`] — who produced the change (a human, an agent, or an
//!   operator/agent pair).
//! - [`contribution::Contribution`] — the proposed change itself, identified by
//!   content digests rather than by any one forge's pull-request number.
//! - [`evidence::Evidence`] — a claim about a contribution, carrying the
//!   [`evidence::EvidenceClass`] that says how much the claim is worth.
//! - [`policy::Policy`] — the repository's own rules, versioned in the repository.
//! - [`verdict::Verdict`] — the reproducible output.
//!
//! [`authorship::AuthorshipClaim`] is not a sixth type either. It rides on an
//! `attested` Evidence record, because a declaration of how a change was
//! produced is exactly a human accepting responsibility for a statement.
//!
//! [`vouch::VouchList`] is not a sixth type. It reads the `VOUCHED.td` file that
//! `mitchellh/vouch` already put in a few hundred repositories, so that the
//! social half of trust does not have to be reinvented here.
//!
//! The class distinction in [`evidence::EvidenceClass`] is the load-bearing idea:
//! a passing test suite and a language model's opinion are both "evidence", and
//! collapsing them into one confidence number is how review budgets get spent on
//! the wrong changes.

pub mod actor;
pub mod authorship;
pub mod contribution;
pub mod evidence;
pub mod policy;
pub mod verdict;
pub mod vouch;

pub use actor::{Actor, ActorKind, TrustTier};
pub use authorship::{AuthorshipClaim, AuthorshipKind};
pub use contribution::Contribution;
pub use evidence::{Evidence, EvidenceClass, Outcome};
pub use policy::Policy;
pub use verdict::{Decision, Verdict};
pub use vouch::{VouchList, VouchStatus};

// The README is the first thing a reader on crates.io sees, and its example was
// not compiled by anything - the same gap `cargo test --doc` had before #65.
// Compiling it under `cfg(doctest)` puts it under test without duplicating the
// crate documentation above.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;
