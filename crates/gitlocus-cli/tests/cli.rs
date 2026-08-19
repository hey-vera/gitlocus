// SPDX-License-Identifier: Apache-2.0
//! What the binary actually does.
//!
//! Adopters do not link `gitlocus-core`; they run `locus`. Everything between a
//! correct evaluator and a correct answer lives in this crate, and until this
//! file existed none of it was tested — mutation testing reported 24 surviving
//! mutants here, including `changed_paths -> Ok(vec![])`, which makes every
//! verdict `satisfied`, and `verify -> Ok(Default::default())`, which makes the
//! exit code unconditionally zero. The exit code is what CI branches on.
//!
//! These tests drive the built binary rather than calling functions, because the
//! exit code and the printed output are the interface being relied on.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A directory that removes itself, so a failing test does not leave litter.
///
/// A dev-dependency would do this better, but `deny.toml` lists exactly the
/// licences already in the tree and says that adding one should be a decision
/// rather than something inherited. This is not worth spending that decision on.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("gitlocus-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).expect("creating a temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).expect("writing a fixture");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn locus() -> Command {
    Command::new(env!("CARGO_BIN_EXE_locus"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A policy demanding one deterministic check and nothing else.
const NEEDS_TESTS: &str = r#"
version: 0
rules:
  - name: baseline
    when:
      paths: ["**"]
    require:
      deterministic: [tests]
      approvals: 0
"#;

fn contribution_json(paths: &str) -> String {
    format!(
        r#"{{"repository":"github.com/acme/repo","base_digest":"aaaa","head_digest":"bbbb",
             "actor":{{"id":"someone","kind":"human","tier":"contributor"}},
             "changed_paths":[{paths}]}}"#
    )
}

const PASSING_TESTS: &str = r#"[{"kind":"tests","class":"deterministic","outcome":"pass",
    "subject_digest":"bbbb","produced_by":"ci","produced_at":"2026-08-19T00:00:00Z"}]"#;

// --- verify ---------------------------------------------------------------

#[test]
fn verify_exits_zero_when_the_policy_is_satisfied() {
    let dir = TempDir::new("verify-ok");
    let policy = dir.write("policy.yml", NEEDS_TESTS);
    let contribution = dir.write("contribution.json", &contribution_json(r#""src/main.rs""#));
    let evidence = dir.write("evidence.json", PASSING_TESTS);

    let out = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--contribution")
        .arg(&contribution)
        .arg("--evidence")
        .arg(&evidence)
        .output()
        .expect("running locus");

    assert!(out.status.success(), "stdout: {}", stdout_of(&out));
    assert!(stdout_of(&out).contains("satisfied"), "{}", stdout_of(&out));
}

#[test]
fn verify_exits_non_zero_when_a_requirement_is_unmet() {
    // The other half of the claim above, and the one that matters: a gate whose
    // exit code is always zero passes every contribution ever proposed.
    let dir = TempDir::new("verify-blocked");
    let policy = dir.write("policy.yml", NEEDS_TESTS);
    let contribution = dir.write("contribution.json", &contribution_json(r#""src/main.rs""#));

    let out = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--contribution")
        .arg(&contribution)
        .output()
        .expect("running locus");

    assert!(!out.status.success(), "stdout: {}", stdout_of(&out));
    assert!(stdout_of(&out).contains("blocked"), "{}", stdout_of(&out));
}

#[test]
fn verify_names_the_rules_that_matched() {
    // `join` renders this line. Replacing it with an empty string leaves a
    // verdict that cannot tell anyone which rule decided it.
    let dir = TempDir::new("verify-rules");
    let policy = dir.write("policy.yml", NEEDS_TESTS);
    let contribution = dir.write("contribution.json", &contribution_json(r#""src/main.rs""#));
    let evidence = dir.write("evidence.json", PASSING_TESTS);

    let out = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--contribution")
        .arg(&contribution)
        .arg("--evidence")
        .arg(&evidence)
        .output()
        .expect("running locus");

    assert!(
        stdout_of(&out).contains("rules matched: baseline"),
        "{}",
        stdout_of(&out)
    );
}

#[test]
fn a_rule_deleted_by_the_contribution_still_governs_it() {
    // The end-to-end form of ADR 0013, through the interface CI actually calls.
    let dir = TempDir::new("verify-governing");
    let shipped = dir.write("head.yml", "version: 0\nrules: []\n");
    let governing = dir.write("base.yml", NEEDS_TESTS);
    let contribution = dir.write(
        "contribution.json",
        &contribution_json(r#"".gitlocus/policy.yml""#),
    );

    let escaped = locus()
        .args(["verify", "--policy"])
        .arg(&shipped)
        .arg("--contribution")
        .arg(&contribution)
        .output()
        .expect("running locus");
    assert!(
        escaped.status.success(),
        "judged by what it ships, it passes"
    );

    let governed = locus()
        .args(["verify", "--policy"])
        .arg(&shipped)
        .arg("--governing-policy")
        .arg(&governing)
        .arg("--contribution")
        .arg(&contribution)
        .output()
        .expect("running locus");

    assert!(!governed.status.success());
    assert!(
        stdout_of(&governed).contains("governing:baseline"),
        "the verdict must name the document the rule came from: {}",
        stdout_of(&governed)
    );
}

#[test]
fn a_governing_policy_that_cannot_be_read_is_fatal_rather_than_ignored() {
    // Silently continuing without the base policy would reopen the hole that
    // passing it closes, and it would do so without saying anything.
    let dir = TempDir::new("verify-missing-base");
    let policy = dir.write("policy.yml", NEEDS_TESTS);
    let contribution = dir.write("contribution.json", &contribution_json(r#""src/main.rs""#));

    let out = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--governing-policy")
        .arg(dir.path().join("does-not-exist.yml"))
        .arg("--contribution")
        .arg(&contribution)
        .output()
        .expect("running locus");

    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("does-not-exist.yml"),
        "the error must name the file it could not read"
    );
}

// --- policy check ---------------------------------------------------------

#[test]
fn policy_check_accepts_a_valid_document_and_rejects_a_malformed_one() {
    let dir = TempDir::new("policy-check");
    let good = dir.write("good.yml", NEEDS_TESTS);
    let bad = dir.write("bad.yml", "version: 0\nrules: [\n");

    let ok = locus()
        .args(["policy", "check", "--policy"])
        .arg(&good)
        .output()
        .expect("running locus");
    assert!(ok.status.success());
    assert!(stdout_of(&ok).contains("ok"));

    let err = locus()
        .args(["policy", "check", "--policy"])
        .arg(&bad)
        .output()
        .expect("running locus");
    assert!(
        !err.status.success(),
        "a malformed policy must not report ok"
    );
}

// --- evidence emit --------------------------------------------------------

#[test]
fn evidence_emit_writes_the_record_and_never_a_signer() {
    // Emitting does not sign. If this ever printed a signer, every signed_by
    // rule in every policy would become decorative.
    let out = locus()
        .args([
            "evidence",
            "emit",
            "--kind",
            "tests",
            "--class",
            "deterministic",
            "--outcome",
            "pass",
            "--subject",
            "bbbb",
            "--produced-by",
            "local",
            "--produced-at",
            "2026-08-19T00:00:00Z",
        ])
        .output()
        .expect("running locus");

    assert!(out.status.success());
    let printed = stdout_of(&out);
    assert!(printed.contains(r#""kind":"tests""#), "{printed}");
    assert!(printed.contains(r#""subject_digest":"bbbb""#), "{printed}");
    assert!(
        !printed.contains("signer"),
        "a signer is a conclusion a verifier reaches, never a claim this emits: {printed}"
    );
}

// --- authorship -----------------------------------------------------------

#[test]
fn a_declaration_is_always_attested_and_never_signed_by_declaring_it() {
    // Class is not a choice here. A deterministic or assessed authorship record
    // would be a machine declaring authorship, which is the thing the mechanism
    // exists to prevent — so the command cannot produce one.
    let out = locus()
        .args([
            "authorship",
            "declare",
            "--claim",
            "human",
            "--subject",
            "bbbb",
            "--by",
            "a-named-human",
            "--produced-at",
            "2026-08-19T00:00:00Z",
        ])
        .output()
        .expect("running locus");

    assert!(out.status.success());
    let printed = stdout_of(&out);
    assert!(printed.contains(r#""class":"attested""#), "{printed}");
    assert!(printed.contains(r#""claim":"human""#), "{printed}");
    assert!(
        printed.contains(r#""produced_by":"a-named-human""#),
        "{printed}"
    );
    assert!(
        !printed.contains("signer"),
        "declaring does not sign: {printed}"
    );
}

#[test]
fn a_derived_claim_without_a_source_is_refused() {
    // A derived claim whose source is unknown says almost nothing, and the
    // source is the whole value of the record to whoever audits the licence.
    let out = locus()
        .args([
            "authorship",
            "declare",
            "--claim",
            "derived",
            "--subject",
            "bbbb",
            "--by",
            "someone",
            "--produced-at",
            "2026-08-19T00:00:00Z",
        ])
        .output()
        .expect("running locus");

    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--source"),
        "the error must say what is missing"
    );
}

#[test]
fn an_undeclared_contribution_is_refused_by_a_human_only_policy() {
    // End to end through the binary: silence is read as generated, so a policy
    // that declines generated work declines a contribution that says nothing.
    let dir = TempDir::new("authorship-e2e");
    let policy = dir.write(
        "policy.yml",
        "version: 0\nrules:\n  - name: licence\n    when:\n      paths: [\"**\"]\n    require:\n      authorship: [human, directed_agent]\n",
    );
    let contribution = dir.write("contribution.json", &contribution_json(r#""src/main.rs""#));

    let silent = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--contribution")
        .arg(&contribution)
        .output()
        .expect("running locus");
    assert!(!silent.status.success(), "{}", stdout_of(&silent));
    assert!(
        stdout_of(&silent).contains("Undeclared"),
        "{}",
        stdout_of(&silent)
    );

    // The same contribution with a declaration a named party is answerable for.
    let declaration = locus()
        .args([
            "authorship",
            "declare",
            "--claim",
            "directed_agent",
            "--subject",
            "bbbb",
            "--by",
            "josh",
            "--produced-at",
            "2026-08-19T00:00:00Z",
        ])
        .output()
        .expect("running locus");
    let evidence = dir.write(
        "evidence.json",
        &format!("[{}]", stdout_of(&declaration).trim()),
    );

    let declared = locus()
        .args(["verify", "--policy"])
        .arg(&policy)
        .arg("--contribution")
        .arg(&contribution)
        .arg("--evidence")
        .arg(&evidence)
        .output()
        .expect("running locus");
    assert!(declared.status.success(), "{}", stdout_of(&declared));
}

// --- vouch ----------------------------------------------------------------

#[test]
fn vouch_check_exits_non_zero_only_for_a_denounced_identity() {
    let dir = TempDir::new("vouch");
    let file = dir.write(
        "VOUCHED.td",
        "github:trusted\n-github:banned  opened 40 unread pull requests\n",
    );

    let vouched = locus()
        .args(["vouch", "check", "--file"])
        .arg(&file)
        .args(["--user", "trusted"])
        .output()
        .expect("running locus");
    assert!(vouched.status.success());
    assert!(
        stdout_of(&vouched).contains("vouched"),
        "{}",
        stdout_of(&vouched)
    );

    let denounced = locus()
        .args(["vouch", "check", "--file"])
        .arg(&file)
        .args(["--user", "banned"])
        .output()
        .expect("running locus");
    assert!(
        !denounced.status.success(),
        "a shell must be able to branch on this without parsing text"
    );
    assert!(
        stdout_of(&denounced).contains("denounced"),
        "{}",
        stdout_of(&denounced)
    );
}

// --- contribution, read from git -----------------------------------------

/// A repository with two commits, the second touching `second.txt`.
fn repo_with_two_commits(label: &str) -> TempDir {
    let dir = TempDir::new(label);
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .expect("running git");
        assert!(
            status.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    };
    git(&["init", "-q"]);
    // Never inherit the ambient identity or signing configuration: a machine
    // with commit.gpgsign set would otherwise fail these tests for no reason.
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "commit.gpgsign", "false"]);

    std::fs::write(dir.path().join("first.txt"), "one").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "first"]);

    std::fs::write(dir.path().join("second.txt"), "two").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "second"]);
    dir
}

#[test]
fn contribution_reads_the_changed_paths_out_of_git() {
    // If this returns an empty list, no rule matches any path and every verdict
    // becomes `satisfied`. It is the single most dangerous thing in the binary.
    let dir = repo_with_two_commits("contribution-paths");

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
            "--actor",
            "someone",
        ])
        .output()
        .expect("running locus");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = stdout_of(&out);
    assert!(
        printed.contains("second.txt"),
        "the changed path must be reported: {printed}"
    );
    assert!(
        !printed.contains("first.txt"),
        "a path this change did not touch must not be: {printed}"
    );
}

#[test]
fn contribution_records_real_digests_for_both_revisions() {
    let dir = repo_with_two_commits("contribution-digests");
    let rev = |r: &str| {
        let out = Command::new("git")
            .args(["rev-parse", r])
            .current_dir(dir.path())
            .output()
            .expect("running git");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
            "--actor",
            "someone",
        ])
        .output()
        .expect("running locus");

    let printed = stdout_of(&out);
    assert!(printed.contains(&rev("HEAD~1")), "base digest: {printed}");
    assert!(printed.contains(&rev("HEAD")), "head digest: {printed}");
}

#[test]
fn contribution_fails_loudly_on_a_revision_that_does_not_exist() {
    // Without this, a git command that failed could be read as success and the
    // contribution would carry a digest of nothing.
    let dir = repo_with_two_commits("contribution-bad-rev");

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "no-such-revision",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
            "--actor",
            "someone",
        ])
        .output()
        .expect("running locus");

    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no-such-revision"),
        "the error must name the revision it could not resolve"
    );
}

#[test]
fn contribution_defaults_the_actor_to_the_head_commit_author() {
    // The path taken when --actor is omitted, which is the path an adopter
    // following the README takes first.
    let dir = repo_with_two_commits("contribution-default-actor");

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
        ])
        .output()
        .expect("running locus");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout_of(&out).contains("test@example.com"),
        "the actor must come from the commit, not from nowhere: {}",
        stdout_of(&out)
    );
}

#[test]
fn contribution_derives_the_repository_from_the_remote() {
    // An identifier invented rather than derived would give two clones of one
    // repository two identities, and the identifier is part of a contribution's
    // identity.
    let dir = repo_with_two_commits("contribution-default-repo");
    let status = Command::new("git")
        .args(["remote", "add", "origin", "git@github.com:acme/widgets.git"])
        .current_dir(dir.path())
        .output()
        .expect("running git");
    assert!(status.status.success());

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--actor",
            "someone",
        ])
        .output()
        .expect("running locus");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout_of(&out).contains("github.com/acme/widgets"),
        "an scp-style remote must normalise to host/owner/name: {}",
        stdout_of(&out)
    );
}

#[test]
fn contribution_records_an_agent_and_its_operator_as_a_pair() {
    let dir = repo_with_two_commits("contribution-pair");

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
            "--actor",
            "someone",
            "--agent",
            "claude-code",
            "--operator",
            "josh",
        ])
        .output()
        .expect("running locus");

    let printed = stdout_of(&out);
    assert!(printed.contains("claude-code"), "{printed}");
    assert!(printed.contains("josh"), "{printed}");
    assert!(
        printed.contains("pair"),
        "an agent with an operator is a pair: {printed}"
    );
}

#[test]
fn a_vouch_raises_an_unknown_actor_but_does_not_demote_a_higher_one() {
    // Both halves of the guard at the vouch branch: an unknown actor is raised,
    // and an actor who already holds a higher tier is left alone.
    let dir = repo_with_two_commits("contribution-vouch");
    std::fs::write(dir.path().join("VOUCHED.td"), "github:someone\n").unwrap();

    let run = |tier: &str| {
        let out = locus()
            .current_dir(dir.path())
            .args([
                "contribution",
                "--base",
                "HEAD~1",
                "--head",
                "HEAD",
                "--repository",
                "github.com/acme/repo",
                "--actor",
                "someone",
                "--tier",
                tier,
                "--vouched-file",
                "VOUCHED.td",
            ])
            .output()
            .expect("running locus");
        stdout_of(&out)
    };

    assert!(
        run("unknown").contains("vouched"),
        "unknown is raised to vouched"
    );
    assert!(
        run("maintainer").contains("maintainer"),
        "an actor who already holds a higher tier is not demoted by appearing in the file"
    );
}

#[test]
fn a_denouncement_caps_the_tier_however_high_it_was_asked_to_be() {
    let dir = repo_with_two_commits("contribution-denounced");
    std::fs::write(dir.path().join("VOUCHED.td"), "-github:someone  spam\n").unwrap();

    let out = locus()
        .current_dir(dir.path())
        .args([
            "contribution",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--repository",
            "github.com/acme/repo",
            "--actor",
            "someone",
            "--tier",
            "maintainer",
            "--vouched-file",
            "VOUCHED.td",
        ])
        .output()
        .expect("running locus");

    assert!(
        stdout_of(&out).contains("unknown"),
        "a denouncement wins over the flag: {}",
        stdout_of(&out)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("denounced"),
        "and it must say so, because a downgrade nobody notices is a bug"
    );
}
