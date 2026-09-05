// SPDX-License-Identifier: AGPL-3.0-only
//! Grants: what a principal let an agent do, and everything it did not.
//!
//! Named as the claims they make. The refusals are the point — a grant that
//! authorises one repository too many, one act too many, or one second too
//! long has failed in exactly the way ADR 0020 §4 exists to prevent.

use gitlocus_core::{Actor, ActorKind, TrustTier};
use locus_ledger::{
    Act, AgentIdentity, Binding, Grant, GrantError, GrantRequest, Refusal, Registry, SCHEMA_VERSION,
};
use proptest::prelude::*;

const REPO: &str = "github.com/hey-vera/gitlocus";
const OTHER: &str = "github.com/acme/widgets";
const T0: i64 = 1_000;
const T1: i64 = 2_000;

fn enrolled() -> (Registry, String) {
    let registry = Registry::in_memory().expect("open");
    let josh = registry
        .enrol(Binding {
            provider: "github".into(),
            subject: "1".into(),
            login: "josh".into(),
        })
        .expect("enrol");
    (registry, josh.id)
}

fn request(principal: &str) -> GrantRequest {
    GrantRequest {
        principal: principal.to_string(),
        agent: AgentIdentity {
            implementation: "claude-code".into(),
            model: Some("claude-fable-5-1".into()),
        },
        repositories: vec![REPO.into()],
        ceiling: TrustTier::Contributor,
        acts: [Act::Propose, Act::Evidence].into_iter().collect(),
        issued_at: T0,
        expires_at: T1,
    }
}

fn issued() -> (Registry, Grant) {
    let (registry, josh) = enrolled();
    let grant = registry.issue(request(&josh)).expect("issue");
    (registry, grant)
}

// --- issuing ----------------------------------------------------------------------

#[test]
fn a_grant_is_issued_to_an_existing_principal_and_read_back() {
    let (registry, grant) = issued();
    let back = registry.grant(&grant.id).expect("read").expect("exists");
    assert_eq!(back, grant);
    assert_eq!(
        back.acts,
        [Act::Propose, Act::Evidence].into_iter().collect()
    );
    assert_eq!(back.repositories, vec![REPO.to_string()]);
    assert_eq!(back.revoked_at, None);
    assert!(back.is_live_at(T0));
    assert!(back.is_live_at(T1 - 1));
    assert!(!back.is_live_at(T1));
}

#[test]
fn a_grant_for_an_unknown_principal_cannot_be_issued() {
    let registry = Registry::in_memory().expect("open");
    match registry.issue(request("nobody")) {
        Err(GrantError::UnknownPrincipal(id)) => assert_eq!(id, "nobody"),
        other => panic!("an unknown principal cannot issue: {other:?}"),
    }
}

#[test]
fn a_grant_with_no_repositories_or_no_acts_cannot_be_issued() {
    let (registry, josh) = enrolled();
    let mut none_where = request(&josh);
    none_where.repositories.clear();
    assert!(matches!(
        registry.issue(none_where),
        Err(GrantError::EmptyScope)
    ));

    let mut nothing = request(&josh);
    nothing.acts.clear();
    assert!(matches!(registry.issue(nothing), Err(GrantError::NoActs)));

    assert!(
        registry.grants_of(&josh).expect("list").is_empty(),
        "nothing was written"
    );
}

#[test]
fn a_grant_that_expires_before_it_is_issued_cannot_be_issued() {
    let (registry, josh) = enrolled();
    let mut dead = request(&josh);
    dead.expires_at = dead.issued_at;
    assert!(matches!(
        registry.issue(dead),
        Err(GrantError::ExpiresBeforeIssue { .. })
    ));
}

#[test]
fn repositories_are_stored_once_each_and_sorted() {
    let (registry, josh) = enrolled();
    let mut req = request(&josh);
    req.repositories = vec![OTHER.into(), REPO.into(), OTHER.into()];
    let grant = registry.issue(req).expect("issue");
    assert_eq!(
        grant.repositories,
        vec![OTHER.to_string(), REPO.to_string()]
    );
}

// --- the delegable set is the type --------------------------------------------------

#[test]
fn a_non_delegable_act_has_no_name_a_grant_can_carry() {
    // ADR 0020 §4: an approval attestation, a legal authorship claim and any
    // change to grants require the principal's own credential. They are not
    // refused by a check; they have no spelling. Every plausible name for one
    // parses to nothing.
    for forbidden in [
        "attest",
        "approve",
        "approval",
        "review",
        "declare_human",
        "declare_directed_agent",
        "declare_derived",
        "issue_grant",
        "revoke_grant",
        "grant",
        "",
        "PROPOSE",
    ] {
        assert_eq!(
            Act::parse(forbidden),
            None,
            "{forbidden} must not be an act"
        );
    }
    // And every delegable act round-trips through its name.
    for act in Act::ALL {
        assert_eq!(Act::parse(act.name()), Some(act));
        assert_eq!(act.to_string(), act.name());
    }
    assert_eq!(
        Act::ALL.len(),
        3,
        "the delegable set is exactly the three in ADR 0020 §4"
    );
}

// --- authorising --------------------------------------------------------------------

#[test]
fn a_live_grant_authorises_what_it_names_and_says_whose_authority_it_is() {
    let (registry, grant) = issued();
    let ok = registry
        .authorise(&grant.id, T0 + 1, REPO, Act::Propose)
        .expect("storage")
        .expect("authorised");
    assert_eq!(ok.principal, grant.principal);
    assert_eq!(ok.agent, grant.agent);
    assert_eq!(ok.hop, grant.hop());
    assert_eq!(ok.hop.ceiling, TrustTier::Contributor);
    assert_eq!(ok.hop.grant.as_deref(), Some(grant.id.as_str()));
}

#[test]
fn a_grant_authorises_only_the_repositories_it_names() {
    let (registry, grant) = issued();
    let refused = registry
        .authorise(&grant.id, T0 + 1, OTHER, Act::Propose)
        .expect("storage")
        .unwrap_err();
    assert_eq!(
        refused,
        Refusal::OutOfScope {
            repository: OTHER.to_string()
        }
    );
}

#[test]
fn a_grant_authorises_only_the_acts_it_names() {
    let (registry, grant) = issued();
    let refused = registry
        .authorise(&grant.id, T0 + 1, REPO, Act::DeclareGenerated)
        .expect("storage")
        .unwrap_err();
    assert_eq!(
        refused,
        Refusal::ActNotGranted {
            act: Act::DeclareGenerated
        }
    );
}

#[test]
fn an_expired_grant_is_refused_however_valid_it_was() {
    let (registry, grant) = issued();
    assert!(
        registry
            .authorise(&grant.id, T1 - 1, REPO, Act::Propose)
            .expect("storage")
            .is_ok(),
        "one second before expiry it works"
    );
    let refused = registry
        .authorise(&grant.id, T1, REPO, Act::Propose)
        .expect("storage")
        .unwrap_err();
    assert_eq!(refused, Refusal::Expired { at: T1 });
}

#[test]
fn a_grant_is_not_in_force_before_it_is_issued() {
    let (registry, grant) = issued();
    let refused = registry
        .authorise(&grant.id, T0 - 1, REPO, Act::Propose)
        .expect("storage")
        .unwrap_err();
    assert_eq!(refused, Refusal::NotYetIssued { at: T0 });
}

#[test]
fn an_unknown_grant_is_refused() {
    let (registry, _) = enrolled();
    let refused = registry
        .authorise("nothing", T0, REPO, Act::Propose)
        .expect("storage")
        .unwrap_err();
    assert_eq!(refused, Refusal::UnknownGrant("nothing".into()));
}

// --- revocation ---------------------------------------------------------------------

#[test]
fn a_revoked_grant_refuses_the_next_request() {
    let (registry, grant) = issued();
    let revoked = registry.revoke(&grant.id, T0 + 10).expect("revoke");
    assert_eq!(revoked.revoked_at, Some(T0 + 10));
    assert!(!revoked.is_live_at(T0 + 11));

    let refused = registry
        .authorise(&grant.id, T0 + 11, REPO, Act::Propose)
        .expect("storage")
        .unwrap_err();
    assert_eq!(refused, Refusal::Revoked { at: T0 + 10 });

    // Revocation wins over every other reason: an in-scope, in-time request
    // hears "revoked", not something that would change if it waited.
    let still = registry
        .authorise(&grant.id, T1 + 100, OTHER, Act::DeclareGenerated)
        .expect("storage")
        .unwrap_err();
    assert_eq!(still, Refusal::Revoked { at: T0 + 10 });
}

#[test]
fn revoking_a_revoked_grant_says_so_and_keeps_the_first_time() {
    let (registry, grant) = issued();
    registry.revoke(&grant.id, T0 + 10).expect("revoke");
    match registry.revoke(&grant.id, T0 + 20) {
        Err(GrantError::AlreadyRevoked { grant: id, at }) => {
            assert_eq!(id, grant.id);
            assert_eq!(at, T0 + 10);
        }
        other => panic!("a second revocation must not move the time: {other:?}"),
    }
    assert_eq!(
        registry
            .grant(&grant.id)
            .expect("read")
            .expect("exists")
            .revoked_at,
        Some(T0 + 10)
    );
}

#[test]
fn revoking_an_unknown_grant_says_so() {
    let (registry, _) = enrolled();
    assert!(matches!(
        registry.revoke("nothing", T0),
        Err(GrantError::UnknownGrant(id)) if id == "nothing"
    ));
}

// --- listing ------------------------------------------------------------------------

#[test]
fn a_principals_grants_are_listed_newest_first_including_revoked_ones() {
    let (registry, josh) = enrolled();
    let older = registry.issue(request(&josh)).expect("issue");
    let mut later = request(&josh);
    later.issued_at = T0 + 100;
    later.expires_at = T1 + 100;
    let newer = registry.issue(later).expect("issue");
    registry.revoke(&older.id, T0 + 50).expect("revoke");

    let listed = registry.grants_of(&josh).expect("list");
    assert_eq!(
        listed.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
        vec![newer.id.as_str(), older.id.as_str()]
    );
    // A revocation somebody cannot see is one they cannot trust.
    assert_eq!(listed[1].revoked_at, Some(T0 + 50));

    let (_, stranger) = enrolled();
    assert!(registry.grants_of(&stranger).expect("list").is_empty());
}

// --- the ceiling reaches the evaluator as a ceiling ------------------------------------

fn tier() -> impl Strategy<Value = TrustTier> {
    prop_oneof![
        Just(TrustTier::Unknown),
        Just(TrustTier::Vouched),
        Just(TrustTier::Contributor),
        Just(TrustTier::Maintainer),
    ]
}

proptest! {
    /// ADR 0020 §2 and #73/#74's line: nothing in the ledger reaches
    /// `Actor::tier` upward. For every tier the root holds and every ceiling a
    /// grant names, the actor built from the grant's hop holds at most both.
    #[test]
    fn a_grant_never_widens_what_its_issuer_could_do(root in tier(), ceiling in tier()) {
        let (registry, josh) = enrolled();
        let mut req = request(&josh);
        req.ceiling = ceiling;
        let grant = registry.issue(req).expect("issue");

        let actor = Actor {
            id: josh.clone(),
            kind: ActorKind::Pair {
                implementation: grant.agent.implementation.clone(),
                model: grant.agent.model.clone(),
                operator: josh.clone(),
            },
            tier: root,
            key_binding: None,
            delegation: vec![grant.hop()],
        };
        let effective = actor.effective_tier();
        prop_assert!(effective <= root, "{effective:?} above the root {root:?}");
        prop_assert!(effective <= ceiling, "{effective:?} above the ceiling {ceiling:?}");
        prop_assert_eq!(effective, root.min(ceiling));
    }
}

// --- migrations ---------------------------------------------------------------------

#[test]
fn the_previous_schema_migrates_forward_with_its_data_intact() {
    // A database written by the previous build - schema 1, principals and
    // bindings only - opens under this one, gains the grants tables, and keeps
    // every row it had. This is the promise crates/locus-ledger/AGENTS.md makes.
    let dir = std::env::temp_dir().join("locus-ledger-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("v1-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let conn = rusqlite::Connection::open(&path).expect("raw open");
        conn.execute_batch(
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
             CREATE INDEX bindings_by_principal ON bindings(principal);
             INSERT INTO principals (id) VALUES ('p1');
             INSERT INTO bindings (provider, subject, login, principal) VALUES ('github', '1', 'josh', 'p1');
             PRAGMA user_version = 1;",
        )
        .expect("write the previous shape");
    }
    let registry = Registry::open(&path).expect("open under the current build");
    assert_eq!(registry.schema_version().expect("version"), SCHEMA_VERSION);
    let josh = registry
        .resolve("github", "1")
        .expect("resolve")
        .expect("the old row survived");
    assert_eq!(josh.id, "p1");
    // And the new tables work against the old principal.
    let grant = registry
        .issue(request("p1"))
        .expect("issue against a migrated principal");
    assert_eq!(registry.grants_of("p1").expect("list"), vec![grant]);
}

// --- errors and refusals name what went wrong ---------------------------------------------

#[test]
fn every_grant_error_and_refusal_names_what_went_wrong() {
    let (registry, grant) = issued();
    let texts = [
        GrantError::UnknownPrincipal("p9".into()).to_string(),
        GrantError::EmptyScope.to_string(),
        GrantError::NoActs.to_string(),
        GrantError::ExpiresBeforeIssue {
            issued_at: 5,
            expires_at: 3,
        }
        .to_string(),
        GrantError::UnknownGrant("g9".into()).to_string(),
        GrantError::AlreadyRevoked {
            grant: "g9".into(),
            at: 7,
        }
        .to_string(),
    ];
    assert!(texts[0].contains("p9"), "{}", texts[0]);
    assert!(texts[1].contains("repository"), "{}", texts[1]);
    assert!(texts[2].contains("act"), "{}", texts[2]);
    assert!(
        texts[3].contains('5') && texts[3].contains('3'),
        "{}",
        texts[3]
    );
    assert!(texts[4].contains("g9"), "{}", texts[4]);
    assert!(
        texts[5].contains("g9") && texts[5].contains('7'),
        "{}",
        texts[5]
    );

    let refusals = [
        Refusal::UnknownGrant("g9".into()).to_string(),
        Refusal::Revoked { at: 11 }.to_string(),
        Refusal::Expired { at: 12 }.to_string(),
        Refusal::NotYetIssued { at: 13 }.to_string(),
        Refusal::OutOfScope {
            repository: OTHER.into(),
        }
        .to_string(),
        Refusal::ActNotGranted {
            act: Act::DeclareGenerated,
        }
        .to_string(),
    ];
    assert!(refusals[0].contains("g9"), "{}", refusals[0]);
    assert!(refusals[1].contains("11"), "{}", refusals[1]);
    assert!(refusals[2].contains("12"), "{}", refusals[2]);
    assert!(refusals[3].contains("13"), "{}", refusals[3]);
    assert!(refusals[4].contains(OTHER), "{}", refusals[4]);
    assert!(refusals[5].contains("declare_generated"), "{}", refusals[5]);

    // A storage error carries the database's own message through.
    let storage = registry.grant(&grant.id).map(|_| ()).err();
    assert!(storage.is_none(), "reading a good grant is not an error");
}
