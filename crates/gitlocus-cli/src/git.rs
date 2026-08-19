// SPDX-License-Identifier: Apache-2.0
//! Reading the facts a contribution is built from out of a git repository.
//!
//! This lives in the CLI rather than in `gitlocus-core` on purpose: the core is a
//! pure function of its inputs, and anything that touches a filesystem, a clock
//! or a subprocess would make its determinism untestable. See
//! `docs/adr/0004-rust-core-shared-by-cli-and-server.md`.
//!
//! `git` is invoked as a subprocess rather than linked as a library. A gitoxide
//! dependency would be the better answer once this needs to read objects, but
//! everything here is available from three plumbing commands, and a subprocess
//! that a contributor can run by hand is easier to trust than a library call
//! they cannot.

use anyhow::{Context, Result, bail};
use std::process::Command;

/// Run a git command and return its trimmed stdout.
fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("running: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8(output.stdout)
        .context("git produced output that is not UTF-8")?
        .trim()
        .to_string())
}

/// Resolve a revision to a full commit digest.
///
/// # Errors
/// Fails when the revision does not name a commit in this repository.
pub fn resolve(revision: &str) -> Result<String> {
    // The ^{commit} peel means a tag resolves to what it points at rather than
    // to the tag object, so the digest identifies the same tree either way.
    git(&[
        "rev-parse",
        "--verify",
        "--end-of-options",
        &format!("{revision}^{{commit}}"),
    ])
    .with_context(|| format!("resolving revision {revision}"))
}

/// Paths that differ between two revisions.
///
/// # Errors
/// Fails when either revision cannot be read.
pub fn changed_paths(base: &str, head: &str) -> Result<Vec<String>> {
    // Two dots, not three: the comparison is between the two revisions as they
    // are. A three-dot diff would compare against the merge base, which silently
    // omits anything the base branch gained since the contribution started - and
    // those are exactly the paths a policy most wants to see.
    let out = git(&["diff", "--name-only", "--end-of-options", base, head])?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

/// Email of the author of a revision.
///
/// # Errors
/// Fails when the revision cannot be read.
pub fn author_email(revision: &str) -> Result<String> {
    git(&["log", "-1", "--format=%ae", "--end-of-options", revision])
}

/// Canonical repository identifier derived from a remote URL.
///
/// # Errors
/// Fails when the remote does not exist or cannot be interpreted.
pub fn repository_from_remote(remote: &str) -> Result<String> {
    let url = git(&["remote", "get-url", "--end-of-options", remote])
        .with_context(|| format!("reading remote {remote}"))?;
    normalise_remote(&url)
        .with_context(|| format!("interpreting remote URL {url}; pass --repository instead"))
}

/// Remove the transport prefix, leaving `host[:port]/path`.
///
/// A local path or a `file://` URL returns `None` rather than being guessed at:
/// there is no host to build a stable identifier from, and inventing one would
/// give two clones of the same repository two different identities.
fn strip_scheme(url: &str) -> Option<String> {
    // scp-style: git@github.com:owner/repo.git — the colon is a path separator
    // here, not a port, so it is rewritten before ports are considered.
    if let Some(rest) = url.strip_prefix("git@") {
        return Some(rest.replacen(':', "/", 1));
    }
    for prefix in ["ssh://git@", "ssh://", "https://", "http://", "git://"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

/// Reduce a remote URL to `host/owner/name`.
///
/// The identifier has to be stable across the several ways the same repository
/// can be addressed, because it is part of a contribution's identity: an SSH
/// clone and an HTTPS clone must not produce two different contributions for
/// one change.
fn normalise_remote(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = strip_scheme(url)?;

    // Credentials in a remote URL must never reach the identifier: it is
    // written into evidence and published.
    let rest = rest
        .rsplit_once('@')
        .map_or(rest.as_str(), |(_, after)| after);
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let rest = rest.strip_suffix(".git").unwrap_or(rest);

    // Drop any port, so ssh://git@host:22/owner/repo matches https://host/owner/repo.
    let mut segments = rest.split('/').filter(|s| !s.is_empty());
    let host = segments.next()?;
    let host = host.split_once(':').map_or(host, |(h, _)| h);
    let path: Vec<&str> = segments.collect();
    if host.is_empty() || path.is_empty() {
        return None;
    }

    Some(format!("{}/{}", host.to_ascii_lowercase(), path.join("/")))
}

#[cfg(test)]
mod tests {
    use super::normalise_remote;

    #[test]
    fn every_way_of_addressing_one_repository_gives_one_identifier() {
        let expected = Some("github.com/hey-vera/gitlocus".to_string());
        for url in [
            "https://github.com/hey-vera/gitlocus.git",
            "https://github.com/hey-vera/gitlocus",
            "https://github.com/hey-vera/gitlocus/",
            "http://github.com/hey-vera/gitlocus.git",
            "git@github.com:hey-vera/gitlocus.git",
            "ssh://git@github.com/hey-vera/gitlocus.git",
            "git://github.com/hey-vera/gitlocus.git",
            "  https://github.com/hey-vera/gitlocus.git  ",
        ] {
            assert_eq!(normalise_remote(url), expected, "for {url}");
        }
    }

    #[test]
    fn credentials_never_reach_the_identifier() {
        // This string is written into evidence and published.
        assert_eq!(
            normalise_remote("https://user:ghp_secrettoken@github.com/hey-vera/gitlocus.git"),
            Some("github.com/hey-vera/gitlocus".to_string())
        );
    }

    #[test]
    fn ports_are_dropped_so_ssh_and_https_agree() {
        assert_eq!(
            normalise_remote("ssh://git@example.com:2222/team/project.git"),
            Some("example.com/team/project".to_string())
        );
    }

    #[test]
    fn nested_paths_are_preserved_for_forges_that_use_them() {
        assert_eq!(
            normalise_remote("https://gitlab.com/group/subgroup/project.git"),
            Some("gitlab.com/group/subgroup/project".to_string())
        );
    }

    #[test]
    fn the_host_is_lowercased_but_the_path_is_not() {
        // Hosts are case-insensitive; repository names are not.
        assert_eq!(
            normalise_remote("https://GitHub.com/Hey-Vera/GitLocus.git"),
            Some("github.com/Hey-Vera/GitLocus".to_string())
        );
    }

    #[test]
    fn uninterpretable_remotes_are_rejected_rather_than_guessed() {
        for url in [
            "",
            "not a url",
            "/local/path",
            "file:///srv/repo.git",
            "https://",
            "https://host",
        ] {
            assert_eq!(normalise_remote(url), None, "for {url:?}");
        }
    }
}
