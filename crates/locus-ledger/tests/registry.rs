// SPDX-License-Identifier: AGPL-3.0-only
//! The principal registry's two invariants, and the migrations under them.
//!
//! Named as the claims they make. The negative ones are the point: a registry
//! that lets one login carry two standings, or strands a principal with no way
//! in, has failed in a way nothing downstream can notice.

use locus_ledger::{Binding, Registry, RegistryError, SCHEMA_VERSION};
use std::path::PathBuf;

fn github(subject: &str, login: &str) -> Binding {
    Binding {
        provider: "github".into(),
        subject: subject.into(),
        login: login.into(),
    }
}

fn gitlab(subject: &str, login: &str) -> Binding {
    Binding {
        provider: "gitlab".into(),
        subject: subject.into(),
        login: login.into(),
    }
}

/// A database file that exists only for one test, so a file-backed path can be
/// opened twice — which an in-memory database cannot.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("locus-ledger-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{name}-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

// --- migrations ------------------------------------------------------------------

#[test]
fn a_fresh_database_is_migrated_to_the_current_schema() {
    let registry = Registry::in_memory().expect("open");
    assert_eq!(registry.schema_version().expect("version"), SCHEMA_VERSION);
}

#[test]
fn opening_an_up_to_date_database_changes_nothing() {
    let path = scratch("reopen");
    {
        let registry = Registry::open(&path).expect("first open");
        registry.enrol(github("1", "josh")).expect("enrol");
    }
    let registry = Registry::open(&path).expect("second open");
    assert_eq!(registry.schema_version().expect("version"), SCHEMA_VERSION);
    let found = registry
        .resolve("github", "1")
        .expect("resolve")
        .expect("still there");
    assert_eq!(found.bindings, vec![github("1", "josh")]);
}

#[test]
fn a_database_from_the_future_is_refused_rather_than_read() {
    // A newer build wrote this file. Reading it with this build's assumptions
    // could silently misinterpret the one asset that cannot be reconstructed,
    // so the only safe answer is no.
    let path = scratch("future");
    {
        let conn = rusqlite::Connection::open(&path).expect("raw open");
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .expect("set version");
    }
    match Registry::open(&path) {
        Err(RegistryError::SchemaFromTheFuture { found }) => {
            assert_eq!(found, SCHEMA_VERSION + 1);
        }
        other => panic!("a future schema must be refused, got {other:?}"),
    }
}

// --- one identity, one principal -------------------------------------------------

#[test]
fn enrolling_creates_a_principal_that_resolves_from_its_binding() {
    let registry = Registry::in_memory().expect("open");
    let created = registry.enrol(github("1", "josh")).expect("enrol");
    let resolved = registry
        .resolve("github", "1")
        .expect("resolve")
        .expect("the binding resolves");
    assert_eq!(resolved, created);
    assert_eq!(created.bindings.len(), 1);
    // The identifier is GitLocus's, not the upstream's: a principal that
    // outlives its first login must not be named after it.
    assert_ne!(created.id, "1");
    assert_ne!(created.id, "josh");
    assert_eq!(created.id.len(), 32, "128 bits of hex: {}", created.id);
}

#[test]
fn a_binding_cannot_belong_to_two_principals() {
    let registry = Registry::in_memory().expect("open");
    let first = registry.enrol(github("1", "josh")).expect("enrol");

    // Not by enrolling it again as a stranger...
    match registry.enrol(github("1", "josh-renamed")) {
        Err(RegistryError::AlreadyBound { principal, .. }) => assert_eq!(principal, first.id),
        other => panic!("a bound identity cannot be enrolled twice: {other:?}"),
    }

    // ...and not by binding it to a second principal either.
    let second = registry
        .enrol(gitlab("9", "someone"))
        .expect("enrol another");
    match registry.bind(&second.id, &github("1", "josh")) {
        Err(RegistryError::AlreadyBound { principal, .. }) => assert_eq!(principal, first.id),
        other => panic!("a bound identity cannot be bound elsewhere: {other:?}"),
    }
    // Nothing moved.
    assert_eq!(
        registry
            .resolve("github", "1")
            .expect("resolve")
            .expect("still bound")
            .id,
        first.id
    );
}

#[test]
fn an_unknown_principal_cannot_be_bound_to_or_unbound_from() {
    let registry = Registry::in_memory().expect("open");
    assert!(matches!(
        registry.bind("nobody", &github("1", "josh")),
        Err(RegistryError::UnknownPrincipal(id)) if id == "nobody"
    ));
    assert!(matches!(
        registry.unbind("nobody", "github", "1"),
        Err(RegistryError::UnknownPrincipal(id)) if id == "nobody"
    ));
    assert!(registry.get("nobody").expect("get").is_none());
}

// --- a principal outlives any one login -------------------------------------------

#[test]
fn two_upstream_identities_bound_to_one_principal_resolve_to_it() {
    // Standing attaches to the principal (ADR 0020 §1). Two logins resolving to
    // one principal is what lets that standing survive leaving either upstream.
    let registry = Registry::in_memory().expect("open");
    let josh = registry.enrol(github("1", "josh")).expect("enrol");
    let josh = registry.bind(&josh.id, &gitlab("9", "josh")).expect("bind");
    assert_eq!(josh.bindings.len(), 2);

    let via_github = registry
        .resolve("github", "1")
        .expect("resolve")
        .expect("bound");
    let via_gitlab = registry
        .resolve("gitlab", "9")
        .expect("resolve")
        .expect("bound");
    assert_eq!(via_github.id, josh.id);
    assert_eq!(via_gitlab.id, josh.id);
    assert_eq!(via_github, via_gitlab);
}

#[test]
fn a_principal_survives_unbinding_one_of_two_upstreams() {
    let registry = Registry::in_memory().expect("open");
    let josh = registry.enrol(github("1", "josh")).expect("enrol");
    registry.bind(&josh.id, &gitlab("9", "josh")).expect("bind");

    let after = registry
        .unbind(&josh.id, "github", "1")
        .expect("unbind one of two");
    assert_eq!(after.id, josh.id);
    assert_eq!(after.bindings, vec![gitlab("9", "josh")]);

    // The removed identity is a stranger again; the remaining one still works.
    assert!(registry.resolve("github", "1").expect("resolve").is_none());
    assert_eq!(
        registry
            .resolve("gitlab", "9")
            .expect("resolve")
            .expect("still bound")
            .id,
        josh.id
    );
}

#[test]
fn the_last_binding_cannot_be_removed() {
    // A principal with no way to log in is unreachable and unanswerable, and
    // ADR 0020 gives it no recovery flow: no passwords, no reset. So the last
    // door stays open, whatever is asked.
    let registry = Registry::in_memory().expect("open");
    let josh = registry.enrol(github("1", "josh")).expect("enrol");

    match registry.unbind(&josh.id, "github", "1") {
        Err(RegistryError::LastBinding { principal }) => assert_eq!(principal, josh.id),
        other => panic!("the last binding must be refused: {other:?}"),
    }
    // And nothing was removed on the way to refusing.
    assert_eq!(
        registry
            .get(&josh.id)
            .expect("get")
            .expect("exists")
            .bindings,
        vec![github("1", "josh")]
    );
}

#[test]
fn unbinding_an_identity_that_was_never_bound_says_so() {
    let registry = Registry::in_memory().expect("open");
    let josh = registry.enrol(github("1", "josh")).expect("enrol");
    registry.bind(&josh.id, &gitlab("9", "josh")).expect("bind");
    match registry.unbind(&josh.id, "github", "2") {
        Err(RegistryError::NotBound {
            principal,
            provider,
            subject,
        }) => {
            assert_eq!(
                (principal.as_str(), provider.as_str(), subject.as_str()),
                (josh.id.as_str(), "github", "2")
            );
        }
        other => panic!("an identity that is not bound cannot be unbound: {other:?}"),
    }
}

#[test]
fn a_binding_refused_by_another_principal_leaves_both_intact() {
    // Every mutation runs in one transaction; a refusal on the way must not
    // leave a half-written row behind.
    let registry = Registry::in_memory().expect("open");
    let a = registry.enrol(github("1", "a")).expect("enrol a");
    let b = registry.enrol(github("2", "b")).expect("enrol b");
    assert!(registry.bind(&b.id, &github("1", "a")).is_err());
    assert_eq!(
        registry.get(&a.id).expect("get").expect("a").bindings.len(),
        1
    );
    assert_eq!(
        registry.get(&b.id).expect("get").expect("b").bindings.len(),
        1
    );
}

// --- an error names what went wrong -----------------------------------------------

#[test]
fn every_error_names_what_went_wrong() {
    // The message is the only diagnostic an operator or a caller ever sees. A
    // Display that could render nothing was the one mutant the suite missed on
    // the first run of this crate, which is exactly the kind of gap this test
    // exists to close: each variant must carry the identifiers that make it
    // actionable.
    let registry = Registry::in_memory().expect("open");
    let josh = registry.enrol(github("1", "josh")).expect("enrol");

    let last = registry
        .unbind(&josh.id, "github", "1")
        .unwrap_err()
        .to_string();
    assert!(
        last.contains(&josh.id) && last.contains("one binding left"),
        "{last}"
    );

    let bound = registry
        .enrol(github("1", "again"))
        .unwrap_err()
        .to_string();
    assert!(
        bound.contains("github:1") && bound.contains(&josh.id),
        "{bound}"
    );

    let unknown = registry.get("nobody").expect("get");
    assert!(unknown.is_none());
    let unknown = registry
        .unbind("nobody", "github", "1")
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("nobody"), "{unknown}");

    registry.bind(&josh.id, &gitlab("9", "josh")).expect("bind");
    let not_bound = registry
        .unbind(&josh.id, "github", "2")
        .unwrap_err()
        .to_string();
    assert!(
        not_bound.contains("github:2") && not_bound.contains(&josh.id),
        "{not_bound}"
    );

    let path = scratch("future-message");
    {
        let conn = rusqlite::Connection::open(&path).expect("raw open");
        conn.pragma_update(None, "user_version", 99)
            .expect("set version");
    }
    let future = Registry::open(&path).unwrap_err().to_string();
    assert!(
        future.contains("99") && future.contains(&SCHEMA_VERSION.to_string()),
        "{future}"
    );
}
