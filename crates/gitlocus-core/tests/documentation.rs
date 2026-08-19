// SPDX-License-Identifier: Apache-2.0
//! The examples in this crate's documentation, parsed.
//!
//! `ci.yml` ran `cargo test --doc` for a while under a comment saying "the
//! model's documentation carries examples worth keeping honest". It reported
//! `running 0 tests`: every fenced block in a doc comment is annotated `yaml` or
//! `text`, and rustdoc compiles neither. The claim was true about the intent and
//! false about the mechanism, which is the shape of failure LEARNINGS.md is a
//! list of.
//!
//! The examples are YAML because a policy is YAML, and rewriting them as Rust so
//! that rustdoc would run them would make the documentation worse to read in
//! order to make a tool happy. So they stay YAML, they are now complete policies
//! rather than fragments — copy-pasteable, which they were not — and this file
//! is what keeps them honest instead.

use gitlocus_core::Policy;

/// Every ```yaml block in the crate's own source, with the doc-comment markers
/// stripped.
fn yaml_examples(source: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(body) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        else {
            continue;
        };
        let body = body.strip_prefix(' ').unwrap_or(body);
        match (&mut current, body.trim_end()) {
            (None, "```yaml") => current = Some(String::new()),
            (Some(_), "```") => examples.push(current.take().expect("open block")),
            (Some(block), _) => {
                block.push_str(body);
                block.push('\n');
            }
            (None, _) => {}
        }
    }
    assert!(current.is_none(), "an unterminated ```yaml block");
    examples
}

/// A policy example that does not parse is worse than no example: it is read as
/// authoritative and copied.
#[test]
fn every_yaml_example_in_the_documentation_is_a_valid_policy() {
    let sources = [
        ("policy.rs", include_str!("../src/policy.rs")),
        ("lib.rs", include_str!("../src/lib.rs")),
        ("authorship.rs", include_str!("../src/authorship.rs")),
        ("evidence.rs", include_str!("../src/evidence.rs")),
        ("actor.rs", include_str!("../src/actor.rs")),
        ("verdict.rs", include_str!("../src/verdict.rs")),
        ("contribution.rs", include_str!("../src/contribution.rs")),
    ];

    let mut checked = 0;
    for (name, source) in sources {
        for (n, example) in yaml_examples(source).into_iter().enumerate() {
            let parsed = Policy::from_yaml(&example)
                .unwrap_or_else(|e| panic!("{name} example {n} does not parse: {e}\n{example}"));
            // Parsing is not enough. A policy with a malformed glob or an
            // unsupported version deserialises happily and fails at the point of
            // use, which is exactly where an example should not send a reader.
            parsed
                .compile()
                .unwrap_or_else(|e| panic!("{name} example {n} does not compile: {e}\n{example}"));
            checked += 1;
        }
    }
    assert!(
        checked >= 3,
        "found {checked} examples; this test has gone blind"
    );
}
