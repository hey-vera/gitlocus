// SPDX-License-Identifier: Apache-2.0
//! The repository's documentation, under test.
//!
//! AGENTS.md says why this file exists better than a comment here could:
//! claims live in prose, and prose has no CI. Every assertion below is a
//! sentence written somewhere in this repository, converted into something that
//! runs. Where a claim cannot be made executable it is not asserted here — the
//! weaker true statement belongs in the document instead.
//!
//! Each test is named as the claim it makes, per the convention in AGENTS.md.
//! Several of them pass today and would have failed at some point in this
//! repository's short history; those are ratchets, and they are the point.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The repository root, two levels up from this crate's manifest.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Every tracked Markdown file, excluding build output and stale tool runs.
fn markdown_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&root(), &mut out, &|p| {
        p.extension().is_some_and(|e| e == "md")
    });
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, keep: &dyn Fn(&Path) -> bool) {
    // `target` and `mutants.out*` are generated; `.git` is not source. Skipping
    // them by name rather than by asking git keeps this test runnable in a
    // worktree with no index.
    const SKIP: [&str; 4] = ["target", ".git", "mutants.out", "mutants.out.old"];
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !SKIP.contains(&name.as_ref()) {
                walk(&path, out, keep);
            }
        } else if keep(&path) {
            out.push(path);
        }
    }
}

// --- links --------------------------------------------------------------------

/// A broken link in a document is the cheapest possible instance of the failure
/// this repository keeps having: text that describes something which is not
/// there. Nothing caught it before; a rename was enough to introduce one.
#[test]
fn every_relative_link_in_every_markdown_file_resolves() {
    let mut broken = Vec::new();
    for file in markdown_files() {
        let text = fs::read_to_string(&file).expect("read");
        let dir = file.parent().expect("parent").to_path_buf();
        for target in markdown_link_targets(&text) {
            // Anchors are not checked: a heading can be renamed without the link
            // becoming wrong in any way a reader would notice, and asserting
            // otherwise would make headings unrenameable.
            let bare = target.split('#').next().unwrap_or_default();
            if bare.is_empty() {
                continue;
            }
            if !dir.join(bare).exists() {
                broken.push(format!("{} -> {target}", file.display()));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "broken relative links:\n{}",
        broken.join("\n")
    );
}

/// Markdown inline-link targets, skipping absolute URLs and mail links.
fn markdown_link_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ']' && i + 1 < bytes.len() && bytes[i + 1] == '(' {
            let mut j = i + 2;
            let mut target = String::new();
            while j < bytes.len() && bytes[j] != ')' && !bytes[j].is_whitespace() {
                target.push(bytes[j]);
                j += 1;
            }
            if !target.starts_with("http")
                && !target.starts_with('#')
                && !target.starts_with("mailto:")
                && !target.is_empty()
            {
                out.push(target);
            }
            i = j;
        }
        i += 1;
    }
    out
}

// --- the map matches the territory --------------------------------------------

/// AGENTS.md ends with a table telling an agent where everything is. A path in
/// it that does not exist sends the reader somewhere and wastes the attention
/// the table was written to save.
#[test]
fn every_path_in_the_where_things_are_table_exists() {
    let agents = read("AGENTS.md");
    let table = section(&agents, "## Where things are");
    let mut missing = Vec::new();
    let mut found = 0;
    for line in table.lines() {
        let Some(cell) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(path) = cell.split('`').next() else {
            continue;
        };
        found += 1;
        if !root().join(path).exists() {
            missing.push(path.to_string());
        }
    }
    assert!(found >= 5, "the table should have rows; parsed {found}");
    assert!(missing.is_empty(), "paths that do not exist: {missing:?}");
}

/// The text under a heading, up to the next heading of the same level.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("no heading {heading}"))
        + heading.len();
    let rest = &text[start..];
    match rest.find("\n## ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

// --- one definition of every check --------------------------------------------

/// The claim: a required check means the same thing locally and in CI, because
/// the workflow runs the recipe rather than a copy of it.
///
/// This is the test that would have caught the sentence AGENTS.md carried for
/// most of this repository's life — that the four commands it listed were "the
/// same commands CI runs; there is no CI-only step" — when in fact they covered
/// four of fifteen checks and none of them passed `--locked`.
#[test]
fn every_required_check_has_a_job_and_a_recipe() {
    let checks = required_checks();
    assert!(
        checks.len() >= 15,
        "expected the full ruleset; got {}",
        checks.len()
    );

    let recipes = just_recipes();
    let workflows = workflow_sources();

    let mut problems = Vec::new();
    for (context, how) in &checks {
        if !workflows
            .iter()
            .any(|(_, text)| declares_job_named(text, context))
        {
            problems.push(format!(
                "`{context}` is required but no workflow declares a job of that name"
            ));
        }

        if let Some(recipe) = how.strip_prefix("just ") {
            let recipe = recipe.trim();
            if !recipes.contains(recipe) {
                problems.push(format!(
                    "`{context}` maps to `just {recipe}`, which the justfile does not define"
                ));
            }
            if !workflows
                .iter()
                .any(|(_, text)| text.contains(&format!("just {recipe}")))
            {
                problems.push(format!(
                    "`{context}` maps to `just {recipe}`, but no workflow invokes it — the command is defined twice"
                ));
            }
        } else if let Some(reason) = how
            .strip_prefix("partial:")
            .or_else(|| how.strip_prefix("github-only:"))
        {
            assert!(
                reason.trim().len() > 20,
                "`{context}` claims an exemption without saying why: {how:?}"
            );
        } else {
            problems.push(format!("`{context}` has an unrecognised mapping: {how:?}"));
        }
    }
    assert!(
        problems.is_empty(),
        "required checks:\n{}",
        problems.join("\n")
    );
}

/// A workflow declares a job named `context`, allowing for a matrix expression
/// standing in for part of the name — `analyze (${{ matrix.language }})`
/// produces the contexts `analyze (rust)` and `analyze (actions)`.
fn declares_job_named(workflow: &str, context: &str) -> bool {
    workflow
        .lines()
        .filter_map(|l| l.trim().strip_prefix("name: "))
        .any(|name| {
            let name = name.trim();
            if name == context {
                return true;
            }
            // Split on the template expressions and require the literal parts to
            // appear in order. `${{ ... }}` matches anything, including nothing.
            let mut rest = context;
            let mut first = true;
            for literal in split_templates(name) {
                if literal.is_empty() {
                    first = false;
                    continue;
                }
                match rest.find(literal) {
                    Some(0) if first => rest = &rest[literal.len()..],
                    Some(_) if !first => {
                        let at = rest.find(literal).expect("just matched");
                        rest = &rest[at + literal.len()..];
                    }
                    _ => return false,
                }
                first = false;
            }
            // Only a name that actually contained a template may match loosely.
            name.contains("${{") && rest.is_empty()
        })
}

/// The literal fragments of a name, with every `${{ ... }}` removed.
fn split_templates(name: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = name;
    while let Some(open) = rest.find("${{") {
        out.push(&rest[..open]);
        if let Some(close) = rest[open..].find("}}") {
            rest = &rest[open + close + 2..];
        } else {
            rest = "";
            break;
        }
    }
    out.push(rest);
    out
}

/// `.github/required-checks.txt`, as context to mapping.
fn required_checks() -> BTreeMap<String, String> {
    read(".github/required-checks.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let (context, how) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("no `=` in required-checks line: {line:?}"));
            (context.trim().to_string(), how.trim().to_string())
        })
        .collect()
}

/// Every recipe name in the justfile.
fn just_recipes() -> BTreeSet<String> {
    read("justfile")
        .lines()
        .filter(|l| !l.starts_with(char::is_whitespace) && !l.starts_with('#'))
        .filter_map(|l| l.split_once(':'))
        .map(|(head, _)| {
            head.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|name| !name.is_empty() && !name.contains('=') && !name.starts_with('_'))
        .collect()
}

fn workflow_sources() -> Vec<(PathBuf, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<_> = fs::read_dir(&dir)
        .expect("workflows directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .map(|p| {
            let text = fs::read_to_string(&p).expect("read workflow");
            (p, text)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        out.len() >= 5,
        "expected the workflow set; found {}",
        out.len()
    );
    out
}

// --- the decision record ------------------------------------------------------

/// `docs/adr/README.md` says it outright: "A record with only benefits is not a
/// decision, it is an advertisement." Every record must therefore reach the
/// section where the cost is stated.
///
/// Two records reach it under a different heading — 0005 argues with the reader
/// directly, and 0015 shows its findings first — so the assertion is that a
/// cost section exists, not that it is spelled one particular way. Asserting the
/// nicer rule instead of the true one would mean editing two good records to
/// satisfy a test.
#[test]
fn every_adr_states_what_it_costs() {
    let mut problems = Vec::new();
    for path in adr_files() {
        let text = fs::read_to_string(&path).expect("read");
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        for required in ["## Context", "## Decision", "## Consequences"] {
            if !text.contains(required) {
                problems.push(format!("{name}: no {required}"));
            }
        }
        let states_a_cost = [
            "## Alternatives rejected",
            "## If you are here to change this",
        ]
        .iter()
        .any(|h| text.contains(h));
        if !states_a_cost {
            problems.push(format!("{name}: nothing states what the decision costs"));
        }
        if !text.contains("**Status:**") {
            problems.push(format!("{name}: no status line"));
        }
    }
    assert!(
        problems.is_empty(),
        "decision records:\n{}",
        problems.join("\n")
    );
}

/// A record that supersedes or amends another must say so, and so must the one
/// it replaces. A one-way link is how a reader arrives at a settled question
/// through the record that lost, and re-opens it.
#[test]
fn superseding_adrs_link_in_both_directions() {
    let mut problems = Vec::new();
    let records: BTreeMap<String, String> = adr_files()
        .into_iter()
        .map(|p| {
            let number = p
                .file_name()
                .expect("name")
                .to_string_lossy()
                .chars()
                .take(4)
                .collect::<String>();
            (number, fs::read_to_string(&p).expect("read"))
        })
        .collect();

    for (number, text) in &records {
        for (forward, backward) in [("Supersedes:", "superseded by"), ("Amends:", "Amended by:")] {
            let Some(line) = text.lines().find(|l| l.contains(forward)) else {
                continue;
            };
            for other in numbers_in(line) {
                let Some(other_text) = records.get(&other) else {
                    problems.push(format!("{number} names {other}, which does not exist"));
                    continue;
                };
                let refers_back = other_text
                    .lines()
                    .filter(|l| l.to_lowercase().contains(&backward.to_lowercase()))
                    .any(|l| numbers_in(l).contains(number));
                if !refers_back {
                    problems.push(format!(
                        "{number} {forward} {other}, but {other} does not point back"
                    ));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "decision records:\n{}",
        problems.join("\n")
    );
}

/// The index is what a reader arrives at. A record missing from it is a decision
/// nobody will find, which for a project whose anti-drift mechanism *is* the
/// decision record is the same as not having made it.
#[test]
fn every_adr_appears_in_the_index() {
    let index = read("docs/adr/README.md");
    let mut missing = Vec::new();
    for path in adr_files() {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        if !index.contains(&name) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "records absent from docs/adr/README.md: {missing:?}"
    );
}

fn adr_files() -> Vec<PathBuf> {
    let mut out: Vec<_> = fs::read_dir(root().join("docs/adr"))
        .expect("docs/adr")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                n.to_string_lossy()
                    .starts_with(|c: char| c.is_ascii_digit())
            })
        })
        .collect();
    out.sort();
    assert!(
        out.len() >= 16,
        "expected the decision records; found {}",
        out.len()
    );
    out
}

/// The four-digit record numbers named on a line.
fn numbers_in(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    for i in 0..chars.len() {
        if chars[i..].len() >= 4 && chars[i..i + 4].iter().all(char::is_ascii_digit) {
            let before_ok = i == 0 || !chars[i - 1].is_ascii_digit();
            let after_ok = chars.len() == i + 4 || !chars[i + 4].is_ascii_digit();
            if before_ok && after_ok {
                out.push(chars[i..i + 4].iter().collect());
            }
        }
    }
    out
}

// --- one version ---------------------------------------------------------------

/// This repository has shipped a wrong version number twice: the README
/// announced v0.0.1 after v0.0.2 shipped, and `locus --version` on the v0.0.2
/// release reported `0.0.1` because the workspace version was never bumped.
/// `release.yml` now checks the binary against the tag; nothing checked the
/// prose, or the composite action's default ref, which is what an adopter
/// actually pins.
#[test]
fn every_version_string_matches_the_workspace_version() {
    let version = read("Cargo.toml")
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.split('"').next())
        .expect("workspace version")
        .to_string();
    let tag = format!("v{version}");

    let mut wrong = Vec::new();
    for (file, needle) in [
        ("README.md", "**Status: v"),
        ("README.md", "uses: hey-vera/gitlocus@v"),
        ("action.yml", "uses: hey-vera/gitlocus@v"),
        ("action.yml", "default: v"),
    ] {
        let text = read(file);
        let mut seen = 0;
        for line in text.lines() {
            let Some(at) = line.find(needle) else {
                continue;
            };
            seen += 1;
            let after = &line[at + needle.len() - 1..];
            let stated: String = after
                .chars()
                .take_while(|c| *c == 'v' || c.is_ascii_digit() || *c == '.')
                .collect();
            if stated != tag {
                wrong.push(format!(
                    "{file}: {needle:?} says {stated}, workspace is {tag}"
                ));
            }
        }
        assert!(
            seen > 0,
            "{file} no longer contains {needle:?}; this test has gone blind"
        );
    }
    assert!(wrong.is_empty(), "version drift:\n{}", wrong.join("\n"));
}

// --- one licence allow-list ----------------------------------------------------

/// `deny.toml` says the allow-list is "exactly the licences present in the
/// current tree, and no more", so that a dependency arriving under a new licence
/// fails on purpose. `dependency-review` enforces the same rule earlier, on the
/// change rather than on the tree — and enforces it from a second copy of the
/// list, written in a workflow.
///
/// Two copies of a policy is how one of them quietly becomes wrong. This is the
/// check that stops that, and it exists because writing the second copy was
/// otherwise the moment to have known better.
#[test]
fn every_allowed_licence_appears_in_both_places() {
    let deny = read("deny.toml");
    let allow = deny
        .split_once("allow = [")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(list, _)| list)
        .expect("an allow list in deny.toml");
    let from_deny: BTreeSet<String> = allow
        .split(',')
        .filter_map(|entry| entry.split('"').nth(1))
        .map(str::to_string)
        .collect();
    assert!(
        from_deny.len() >= 3,
        "parsed {from_deny:?} from deny.toml; this test has gone blind"
    );

    let workflow = read(".github/workflows/supply-chain.yml");
    let line = workflow
        .lines()
        .find_map(|l| l.trim().strip_prefix("allow-licenses:"))
        .expect("an allow-licenses line in supply-chain.yml");
    let from_workflow: BTreeSet<String> = line
        .split(',')
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect();

    assert_eq!(
        from_deny, from_workflow,
        "deny.toml and dependency-review disagree about which licences are allowed"
    );
}

// --- learnings carry a guard ---------------------------------------------------

/// LEARNINGS.md says a learning without a guard is a note, and notes decay into
/// folklore a new contributor has no reason to read. This asserts that the claim
/// to have one is not empty, and that anything it names exists.
///
/// It cannot assert that a guard is any *good*. Claiming one that does not hold
/// would be the same failure as every row in that table, which is why the forms
/// it accepts are the ones that can be checked: a test that exists, a path that
/// exists, a recipe that exists, an issue, or an explicit `none` with a reason.
/// Two guards written for that file named tests that did not exist, and this is
/// what found them.
#[test]
fn every_learning_has_a_guard() {
    let text = read("LEARNINGS.md");
    let mut problems = Vec::new();

    let mut rows = 0;
    for line in section(&text, "## Claims shipped stronger than the implementation").lines() {
        let cells: Vec<&str> = line.trim().split('|').map(str::trim).collect();
        // A row is `| claimed | actual | guard |`, so splitting gives two empty
        // outer cells. The header and its underline are skipped by name.
        if cells.len() != 5 || cells[1] == "claimed" || cells[1].starts_with("---") {
            continue;
        }
        rows += 1;
        check_guard(
            cells[3],
            &format!("row {:?}", truncate(cells[1])),
            &mut problems,
        );
    }
    assert!(rows >= 16, "parsed {rows} rows; this test has gone blind");

    let traps = section(&text, "## Traps worth not rediscovering");
    let mut heading: Option<String> = None;
    let mut guarded = true;
    let mut traps_seen = 0;
    for line in traps.lines() {
        if let Some(title) = line.strip_prefix("### ") {
            if let Some(previous) = heading.take()
                && !guarded
            {
                problems.push(format!("trap {previous:?} states no guard"));
            }
            heading = Some(truncate(title));
            traps_seen += 1;
            guarded = false;
        } else if let Some(guard) = line.strip_prefix("**Guard:**") {
            guarded = true;
            let name = heading.clone().unwrap_or_default();
            check_guard(guard, &format!("trap {name:?}"), &mut problems);
        }
    }
    if let Some(last) = heading
        && !guarded
    {
        problems.push(format!("trap {last:?} states no guard"));
    }
    assert!(
        traps_seen >= 10,
        "parsed {traps_seen} traps; this test has gone blind"
    );

    assert!(problems.is_empty(), "learnings:\n{}", problems.join("\n"));
}

fn truncate(s: &str) -> String {
    s.chars().take(48).collect()
}

/// A guard names a test, a path, a recipe, an issue, or nothing with a reason.
fn check_guard(guard: &str, what: &str, problems: &mut Vec<String>) {
    let guard = guard.trim();
    if guard.is_empty() {
        problems.push(format!("{what} has an empty guard"));
        return;
    }
    if let Some(reason) = guard.strip_prefix("none") {
        // An em dash, a hyphen or a space may separate `none` from its reason.
        let reason = reason.trim_start_matches(['\u{2014}', '-', ' ']);
        if reason.trim().len() < 10 {
            problems.push(format!(
                "{what} claims no guard is possible without saying why"
            ));
        }
        return;
    }
    if guard.contains("github.com/hey-vera/gitlocus/issues/") {
        return;
    }

    let Some(named) = guard.split('`').nth(1) else {
        problems.push(format!("{what} names nothing checkable: {guard:?}"));
        return;
    };
    if let Some(recipe) = named.strip_prefix("just ") {
        if !just_recipes().contains(recipe.trim()) {
            problems.push(format!(
                "{what} names recipe `{recipe}`, which the justfile does not define"
            ));
        }
        return;
    }
    // A path is tried before a test name because `justfile` is a legitimate
    // guard and looks like an identifier.
    if root().join(named).exists() || any_source_contains(&format!("fn {named}")) {
        return;
    }
    problems.push(format!(
        "{what} names `{named}`, which is neither a path in this repository nor a test in it"
    ));
}

/// Whether any Rust source in the workspace contains `needle`.
fn any_source_contains(needle: &str) -> bool {
    let mut files = Vec::new();
    walk(&root().join("crates"), &mut files, &|p| {
        p.extension().is_some_and(|e| e == "rs")
    });
    files
        .iter()
        .any(|p| fs::read_to_string(p).is_ok_and(|t| t.contains(needle)))
}

// --- what is deliberately not asserted here ------------------------------------
//
// SPDX headers: `just licence-headers` already checks every source file against
// the licence its directory requires, and it is a required status check. A
// second implementation here would be a second thing to keep correct, not a
// second guarantee.
//
// Anchors inside links: see `every_relative_link_in_every_markdown_file_resolves`.
//
// Whether the ruleset still requires exactly the checks in
// `.github/required-checks.txt`: that needs a token, so it lives in `just brief`.
// This suite runs offline and in a pull request's context, where it cannot.
