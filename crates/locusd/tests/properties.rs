// SPDX-License-Identifier: AGPL-3.0-only
//! The public surface, over inputs nobody chose.
//!
//! `route` is reachable by anyone who can reach `https://locus.heyvera.org`. It
//! takes a method, a URL and a body, all three of which are whatever the caller
//! sent. `read_bounded` sits in front of it and is the only thing between the
//! service and a body somebody picked.
//!
//! Eighteen hand-written tests cover what those functions are meant to do. These
//! cover what they must never do, which is panic — a panicking route handler on
//! an unauthenticated endpoint is an availability bug reachable by a stranger.
//!
//! See [ADR 0018](../../../docs/adr/0018-quantified-claims-are-tested-as-properties.md).

use locusd::{read_bounded, route};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// Any method, any URL, any body. A 400 or a 404 is a correct answer; a
    /// panic is not an answer at all.
    #[test]
    fn routing_never_panics(method in ".*", url in ".*", body in prop::collection::vec(any::<u8>(), 0..2048)) {
        let reply = route(&method, &url, &body);
        // A reply nobody can read is as useless as no reply, so the status has
        // to stay inside the range HTTP defines.
        prop_assert!((100..600).contains(&reply.status), "status {} is not an HTTP status", reply.status);
    }

    /// The routes that exist, with bodies that were not written for them. This
    /// is the shape of an actual attempt: the path is right, the payload is not.
    #[test]
    fn a_known_route_with_an_arbitrary_body_never_panics(
        path in prop::sample::select(vec!["/healthz", "/v0/openapi.json", "/v0/verdict"]),
        method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE", "PATCH"]),
        body in prop::collection::vec(any::<u8>(), 0..4096),
    ) {
        let reply = route(method, path, &body);
        prop_assert!((100..600).contains(&reply.status));
    }

    /// Almost-JSON is the interesting case for `/v0/verdict`: bytes that get far
    /// enough into `serde_json` to reach the policy parser and the glob compiler
    /// behind it.
    #[test]
    fn a_verdict_request_that_is_nearly_valid_never_panics(fragment in ".*") {
        let body = format!(
            r#"{{"policy":"{fragment}","contribution":{{"repository":"r","base_digest":"a","head_digest":"b","actor":{{"id":"x","kind":"human","tier":"unknown"}}}},"evidence":[]}}"#
        );
        let reply = route("POST", "/v0/verdict", body.as_bytes());
        prop_assert!((100..600).contains(&reply.status));
    }

    /// The cap holds whatever the caller declares, because the declared length
    /// is the caller's claim about the caller's own request. A body over the cap
    /// must be refused rather than truncated: truncating would hand the
    /// evaluator a document that parses to something nobody sent.
    #[test]
    fn a_body_is_never_truncated_to_something_nobody_sent(
        declared in prop::option::of(0usize..4_000_000),
        body in prop::collection::vec(any::<u8>(), 0..4096),
    ) {
        let mut reader = body.as_slice();
        // Refusing is always allowed; returning something shorter than what was
        // sent is not, because the evaluator would then read a document nobody
        // wrote.
        if let Ok(read) = read_bounded(declared, &mut reader) {
            prop_assert_eq!(read, body);
        }
    }
}
