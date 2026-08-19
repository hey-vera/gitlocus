// SPDX-License-Identifier: AGPL-3.0-only
//! What the service answers.
//!
//! Two layers, because they fail differently. Routing and the body limit are
//! pure and are tested directly — a deleted route answers 404 and a broken limit
//! answers 200, both silently. The socket path is tested once, over real TCP,
//! because nothing else proves the pure parts are actually wired to a listener.

use locusd::{MAX_BODY, Reply, evaluate, read_bounded, route, serve};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const POLICY: &str = "version: 0\nrules:\n  - name: baseline\n    when:\n      paths: [\"**\"]\n    require:\n      deterministic: [tests]\n";
const CONTRIBUTION: &str = r#"{"repository":"github.com/acme/repo","base_digest":"aaaa","head_digest":"bbbb","actor":{"id":"someone","kind":"human","tier":"contributor"},"changed_paths":["src/main.rs"]}"#;

fn request_body(evidence: &str, governing: Option<&str>) -> Vec<u8> {
    let governing = governing.map_or("null".to_string(), |g| {
        serde_json::to_string(g).expect("a string always encodes")
    });
    format!(
        r#"{{"policy":{},"governing_policy":{governing},"contribution":{CONTRIBUTION},"evidence":{evidence}}}"#,
        serde_json::to_string(POLICY).unwrap()
    )
    .into_bytes()
}

fn json(reply: &Reply) -> serde_json::Value {
    serde_json::from_slice(&reply.body).expect("every route answers JSON")
}

// --- evaluation -----------------------------------------------------------

#[test]
fn a_contribution_with_its_evidence_is_satisfied() {
    let evidence = r#"[{"kind":"tests","class":"deterministic","outcome":"pass","subject_digest":"bbbb","produced_by":"ci","produced_at":"2026-08-19T00:00:00Z"}]"#;
    let response = evaluate(&request_body(evidence, None)).expect("valid request");
    assert_eq!(
        response.verdict.decision,
        gitlocus_core::Decision::Satisfied
    );
    assert!(!response.governed_by_base);
}

#[test]
fn a_contribution_without_its_evidence_is_blocked() {
    let response = evaluate(&request_body("[]", None)).expect("valid request");
    assert_eq!(response.verdict.decision, gitlocus_core::Decision::Blocked);
}

#[test]
fn a_signer_sent_in_the_request_is_discarded() {
    // The whole value of a signed_by rule rests on this, and a network boundary
    // is exactly where someone would try it.
    let forged = r#"[{"kind":"tests","class":"deterministic","outcome":"pass","subject_digest":"bbbb","produced_by":"me","produced_at":"2026-08-19T00:00:00Z","signer":"https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"}]"#;
    let response = evaluate(&request_body(forged, None)).expect("valid request");
    let encoded = serde_json::to_string(&response).expect("verdict encodes");
    assert!(
        !encoded.contains("workflows/ci.yml"),
        "a signer must never survive the request boundary: {encoded}"
    );
}

#[test]
fn a_rule_deleted_by_the_contribution_still_governs_it() {
    // ADR 0013 across the network boundary.
    let gutted = serde_json::to_string("version: 0\nrules: []\n").unwrap();
    let body = format!(
        r#"{{"policy":{gutted},"governing_policy":{},"contribution":{CONTRIBUTION},"evidence":[]}}"#,
        serde_json::to_string(POLICY).unwrap()
    );
    let response = evaluate(body.as_bytes()).expect("valid request");
    assert_eq!(response.verdict.decision, gitlocus_core::Decision::Blocked);
    assert!(response.governed_by_base);
    assert!(
        response
            .verdict
            .matched_rules
            .iter()
            .any(|r| r == "governing:baseline"),
        "{:?}",
        response.verdict.matched_rules
    );
}

#[test]
fn a_malformed_policy_is_a_client_error_rather_than_a_verdict() {
    let body = br#"{"policy":"version: 99\nrules: []\n","contribution":{"repository":"r","base_digest":"a","head_digest":"b","actor":{"id":"x","kind":"human","tier":"unknown"},"changed_paths":[]},"evidence":[]}"#;
    let why = evaluate(body).expect_err("must not produce a verdict");
    assert!(why.contains("policy"), "{why}");
}

// --- the body limit -------------------------------------------------------

#[test]
fn the_limit_is_the_one_the_contract_documents() {
    // Pinned because the number is a promise to callers, not an implementation
    // detail: shrinking it silently turns working requests into 413s, and
    // growing it silently raises what one caller can make this box do.
    assert_eq!(MAX_BODY, 1024 * 1024, "one mebibyte");
}

#[test]
fn an_oversized_body_is_refused_rather_than_truncated() {
    // Truncation would hand the evaluator a document the caller did not send,
    // which could parse to something valid and produce a verdict about it.
    let huge = vec![b'x'; MAX_BODY + 1];
    let why = read_bounded(None, &mut huge.as_slice()).expect_err("must refuse");
    assert!(why.contains("exceeds"), "{why}");
}

#[test]
fn a_declared_length_over_the_limit_is_refused_without_reading_it() {
    let why = read_bounded(Some(MAX_BODY + 1), &mut b"short".as_slice()).expect_err("must refuse");
    assert!(why.contains("exceeds"), "{why}");
}

#[test]
fn a_body_at_the_limit_is_accepted() {
    // The boundary in both directions, so the comparison cannot drift to >= or
    // <= without something failing.
    let exact = vec![b'x'; MAX_BODY];
    let read = read_bounded(Some(MAX_BODY), &mut exact.as_slice()).expect("must accept");
    assert_eq!(read.len(), MAX_BODY);
}

#[test]
fn a_lying_declared_length_does_not_get_past_the_cap() {
    // The declared length is the caller's claim about the caller's own request,
    // so the read is capped regardless of what it says.
    let huge = vec![b'x'; MAX_BODY + 1];
    let why = read_bounded(Some(1), &mut huge.as_slice()).expect_err("must refuse");
    assert!(why.contains("exceeds"), "{why}");
}

// --- routing --------------------------------------------------------------

#[test]
fn healthz_reports_the_version_that_is_answering() {
    let reply = route("GET", "/healthz", &[]);
    assert_eq!(reply.status, 200);
    assert_eq!(json(&reply)["status"], "ok");
    assert_eq!(json(&reply)["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn the_contract_is_served_and_describes_every_route() {
    let reply = route("GET", "/v0/openapi.json", &[]);
    assert_eq!(reply.status, 200);
    let doc = json(&reply);
    for path in ["/healthz", "/v0/verdict", "/v0/openapi.json"] {
        assert!(doc["paths"].get(path).is_some(), "{path} is not described");
    }
}

#[test]
fn a_verdict_is_two_hundred_even_when_the_contribution_is_blocked() {
    // The status code describes the request, not the contribution. Conflating
    // them would make a working gate look like a broken service.
    let reply = route("POST", "/v0/verdict", &request_body("[]", None));
    assert_eq!(reply.status, 200);
    assert_eq!(json(&reply)["verdict"]["decision"], "blocked");
}

#[test]
fn an_unreadable_request_is_a_client_error() {
    let reply = route("POST", "/v0/verdict", b"not json");
    assert_eq!(reply.status, 400);
    assert!(json(&reply)["error"].is_string());
}

#[test]
fn a_query_string_does_not_hide_a_route() {
    let reply = route("GET", "/healthz?from=uptime-monitor", &[]);
    assert_eq!(reply.status, 200, "a caller may append a query string");
}

#[test]
fn an_unknown_path_and_an_unsupported_method_are_told_apart() {
    assert_eq!(route("GET", "/nope", &[]).status, 404);
    assert_eq!(route("DELETE", "/healthz", &[]).status, 405);
}

// --- the socket -----------------------------------------------------------

/// Never wait forever on a socket.
///
/// A listener that is bound but never served accepts the connection and answers
/// nothing, so a read with no deadline blocks until the test harness gives up.
/// That turns a clear assertion failure into a hang, which is a worse test even
/// before considering that mutation testing reads a hang as inconclusive rather
/// than as a mutant it caught.
fn deadline(stream: &TcpStream) {
    // Short on purpose. A healthy service answers in milliseconds, so this only
    // ever elapses when something is wrong — and when it does, several of these
    // in one test binary must still add up to less than the budget a mutation
    // run allows, or a caught mutant is reported as inconclusive instead.
    let limit = Some(std::time::Duration::from_secs(2));
    stream
        .set_read_timeout(limit)
        .expect("setting a read timeout");
    stream
        .set_write_timeout(limit)
        .expect("setting a write timeout");
}

/// Raw HTTP over TCP, so the test needs no client dependency and exercises the
/// same bytes a real caller would send.
fn get(addr: std::net::SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connecting to locusd");
    deadline(&stream);
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .expect("writing the request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("reading the response");
    response
}

fn post(addr: std::net::SocketAddr, path: &str, body: &[u8]) -> String {
    let mut stream = TcpStream::connect(addr).expect("connecting to locusd");
    deadline(&stream);
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("writing the request head");
    stream.write_all(body).expect("writing the request body");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("reading the response");
    response
}

#[test]
fn the_routes_are_actually_wired_to_a_listener() {
    // Everything above is pure. This is the one test that fails if the routing
    // is correct and nothing ever calls it.
    let listener = TcpListener::bind("127.0.0.1:0").expect("binding an ephemeral port");
    let addr = listener.local_addr().expect("reading the bound address");
    let server = tiny_http::Server::from_listener(listener, None).expect("wrapping the listener");
    std::thread::spawn(move || serve(&server));

    let response = get(addr, "/healthz");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("application/json"), "{response}");
    assert!(response.contains(r#""status":"ok""#), "{response}");

    let missing = get(addr, "/nope");
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");

    // A POST takes a different path through the handler — it is the only method
    // that reads a body — so a socket test that only ever GETs proves half of it.
    let verdict = post(addr, "/v0/verdict", &request_body("[]", None));
    assert!(verdict.starts_with("HTTP/1.1 200"), "{verdict}");
    assert!(verdict.contains(r#""decision":"blocked""#), "{verdict}");
}

#[test]
fn the_binary_listens_where_locusd_addr_says() {
    // The deployed artifact is the binary, and the address it binds comes from
    // the environment a systemd unit sets. Nothing else here would notice if
    // that wiring broke.
    let probe = TcpListener::bind("127.0.0.1:0").expect("finding a free port");
    let addr = probe.local_addr().expect("reading the port");
    drop(probe);

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_locusd"))
        .env("LOCUSD_ADDR", addr.to_string())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawning locusd");

    // Bounded by total elapsed time, not by an attempt count. Each attempt can
    // burn the whole socket deadline when the service is bound but not
    // answering, and forty of those is eighty seconds — long enough that a
    // mutation run reads a caught mutant as inconclusive.
    let give_up = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut answered = None;
    while std::time::Instant::now() < give_up {
        if let Ok(mut stream) = TcpStream::connect(addr) {
            deadline(&stream);
            write!(
                stream,
                "GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )
            .expect("writing the request");
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() && !response.is_empty() {
                answered = Some(response);
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = child.kill();
    let _ = child.wait();

    let response = answered.expect("locusd never answered on the address it was given");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains(r#""status":"ok""#), "{response}");
}
