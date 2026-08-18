// SPDX-License-Identifier: Apache-2.0
//! Reading `VOUCHED.td`, the trust file used by `mitchellh/vouch`.
//!
//! Vouch already solved the social half of this problem, and it is in production
//! on Ghostty with a few hundred repositories carrying the file. Defining a
//! competing trust format would fragment a thing that works, so GitLocus reads
//! theirs instead and maps it onto [`TrustTier::Vouched`].
//!
//! The format is deliberately trivial — one identifier per line, optionally
//! prefixed with a platform, optionally prefixed with `-` to denounce, with
//! anything after the identifier treated as a human-readable reason:
//!
//! ```text
//! # a comment
//! alice
//! github:bob
//! -github:mallory  opened 40 unread pull requests in a day
//! ```
//!
//! Parsing is total: a line this module cannot interpret is ignored rather than
//! rejected. A trust file that fails closed on a typo would be a denial of
//! service on the project that owns it.

use crate::actor::TrustTier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a trust file says about an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VouchStatus {
    /// Explicitly vouched for.
    Vouched,
    /// Explicitly denounced. Stronger than absence, and never overridden by a
    /// vouch elsewhere in the same file.
    Denounced,
    /// Not mentioned.
    Unknown,
}

/// One entry in a trust file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    denounced: bool,
    reason: Option<String>,
}

/// A parsed `VOUCHED.td`.
///
/// Keys are stored as `platform:username` when a platform was given and as
/// `username` when it was not. A bare entry matches on any platform; a
/// platform-qualified entry matches only that platform.
#[derive(Debug, Clone, Default)]
pub struct VouchList {
    qualified: BTreeMap<(String, String), Entry>,
    bare: BTreeMap<String, Entry>,
}

impl VouchList {
    /// Parse a trust file.
    ///
    /// Never fails. Lines that cannot be interpreted are skipped.
    #[must_use]
    pub fn parse(src: &str) -> Self {
        let mut list = Self::default();

        for raw in src.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (denounced, rest) = match line.strip_prefix('-') {
                Some(rest) => (true, rest.trim_start()),
                None => (false, line),
            };

            let mut parts = rest.splitn(2, char::is_whitespace);
            let Some(identifier) = parts.next().filter(|s| !s.is_empty()) else {
                continue;
            };
            let reason = parts
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);

            let entry = Entry { denounced, reason };

            match identifier.split_once(':') {
                Some((platform, username)) if !platform.is_empty() && !username.is_empty() => {
                    list.insert_qualified(platform, username, entry);
                }
                // A lone colon, or a leading/trailing one, is a malformed entry
                // rather than a bare username. Skipping is safer than guessing.
                Some(_) => {}
                None => list.insert_bare(identifier, entry),
            }
        }

        list
    }

    fn insert_qualified(&mut self, platform: &str, username: &str, entry: Entry) {
        let key = (platform.to_ascii_lowercase(), username.to_ascii_lowercase());
        Self::insert(&mut self.qualified, key, entry);
    }

    fn insert_bare(&mut self, username: &str, entry: Entry) {
        Self::insert(&mut self.bare, username.to_ascii_lowercase(), entry);
    }

    /// A denouncement is never displaced by a later vouch for the same identity.
    /// A file that says both is contradictory, and the safe reading of a
    /// contradiction about trust is the restrictive one.
    fn insert<K: Ord>(map: &mut BTreeMap<K, Entry>, key: K, entry: Entry) {
        match map.get(&key) {
            Some(existing) if existing.denounced => {}
            _ => {
                map.insert(key, entry);
            }
        }
    }

    /// What this file says about an identity on a given platform.
    ///
    /// A platform-qualified entry takes precedence over a bare one, and a
    /// denouncement from either takes precedence over any vouch.
    #[must_use]
    pub fn status(&self, platform: &str, username: &str) -> VouchStatus {
        let platform = platform.to_ascii_lowercase();
        let username = username.to_ascii_lowercase();

        let qualified = self.qualified.get(&(platform, username.clone()));
        let bare = self.bare.get(&username);

        if matches!(qualified, Some(e) if e.denounced) || matches!(bare, Some(e) if e.denounced) {
            return VouchStatus::Denounced;
        }
        if qualified.is_some() || bare.is_some() {
            return VouchStatus::Vouched;
        }
        VouchStatus::Unknown
    }

    /// The stated reason for an entry, if the file gave one.
    #[must_use]
    pub fn reason(&self, platform: &str, username: &str) -> Option<&str> {
        let platform = platform.to_ascii_lowercase();
        let username = username.to_ascii_lowercase();
        self.qualified
            .get(&(platform, username.clone()))
            .or_else(|| self.bare.get(&username))
            .and_then(|e| e.reason.as_deref())
    }

    /// How many identities the file mentions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.qualified.len() + self.bare.len()
    }

    /// Whether the file mentions nobody.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The tier a vouch status justifies on its own.
///
/// Deliberately conservative. A vouch establishes that someone the project
/// already trusts will speak for this person; it does not establish that they
/// have landed work here, so it stops at [`TrustTier::Vouched`]. A denouncement
/// yields [`TrustTier::Unknown`] rather than something lower, because the model
/// has no tier below unknown — the useful consequence of a denouncement is that
/// it *blocks promotion*, which is what returning the floor achieves.
#[must_use]
pub fn tier_for(status: VouchStatus) -> TrustTier {
    match status {
        VouchStatus::Vouched => TrustTier::Vouched,
        VouchStatus::Denounced | VouchStatus::Unknown => TrustTier::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# The Ghostty-style trust file.
alice
github:bob   landed the tree-sitter work
-github:mallory  opened 40 unread pull requests in a day
-carol
  gitlab:dave

:malformed
trailing:
";

    #[test]
    fn bare_entries_match_on_any_platform() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("github", "alice"), VouchStatus::Vouched);
        assert_eq!(list.status("gitlab", "alice"), VouchStatus::Vouched);
        assert_eq!(list.status("codeberg", "alice"), VouchStatus::Vouched);
    }

    #[test]
    fn qualified_entries_match_only_their_platform() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("github", "bob"), VouchStatus::Vouched);
        assert_eq!(list.status("gitlab", "bob"), VouchStatus::Unknown);
    }

    #[test]
    fn denouncements_are_read() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("github", "mallory"), VouchStatus::Denounced);
        assert_eq!(list.status("github", "carol"), VouchStatus::Denounced);
    }

    #[test]
    fn a_denouncement_survives_a_later_vouch_for_the_same_identity() {
        // A file saying both is contradictory. The safe reading of a
        // contradiction about trust is the restrictive one.
        let list = VouchList::parse("-github:x  banned\ngithub:x\n");
        assert_eq!(list.status("github", "x"), VouchStatus::Denounced);

        // ...and in the other order, too.
        let list = VouchList::parse("github:x\n-github:x  banned\n");
        assert_eq!(list.status("github", "x"), VouchStatus::Denounced);
    }

    #[test]
    fn a_bare_denouncement_beats_a_qualified_vouch() {
        let list = VouchList::parse("github:x\n-x\n");
        assert_eq!(list.status("github", "x"), VouchStatus::Denounced);
    }

    #[test]
    fn unmentioned_identities_are_unknown() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("github", "nobody"), VouchStatus::Unknown);
    }

    #[test]
    fn matching_is_case_insensitive() {
        let list = VouchList::parse("GitHub:Bob\n");
        assert_eq!(list.status("github", "bob"), VouchStatus::Vouched);
        assert_eq!(list.status("GITHUB", "BOB"), VouchStatus::Vouched);
    }

    #[test]
    fn reasons_are_preserved() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(
            list.reason("github", "bob"),
            Some("landed the tree-sitter work")
        );
        assert_eq!(
            list.reason("github", "mallory"),
            Some("opened 40 unread pull requests in a day")
        );
        assert_eq!(list.reason("github", "alice"), None);
    }

    #[test]
    fn parsing_is_total() {
        // A trust file that failed closed on a typo would be a denial of service
        // on the project that owns it, so unparseable lines are skipped.
        for src in [
            "",
            "\n\n\n",
            "#only a comment",
            ":",
            "-",
            "- ",
            ":x",
            "x:",
            "---",
        ] {
            let _ = VouchList::parse(src);
        }
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("github", "malformed"), VouchStatus::Unknown);
        assert_eq!(list.status("github", "trailing"), VouchStatus::Unknown);
    }

    #[test]
    fn indented_entries_are_read() {
        let list = VouchList::parse(SAMPLE);
        assert_eq!(list.status("gitlab", "dave"), VouchStatus::Vouched);
    }

    #[test]
    fn a_vouch_stops_at_the_vouched_tier() {
        // A vouch says someone will speak for you. It does not say you have
        // landed work here, so it must not reach contributor.
        assert_eq!(tier_for(VouchStatus::Vouched), TrustTier::Vouched);
        assert!(!tier_for(VouchStatus::Vouched).satisfies(TrustTier::Contributor));
    }

    #[test]
    fn a_denouncement_yields_the_floor() {
        assert_eq!(tier_for(VouchStatus::Denounced), TrustTier::Unknown);
        assert_eq!(tier_for(VouchStatus::Unknown), TrustTier::Unknown);
    }
}
