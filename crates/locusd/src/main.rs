// SPDX-License-Identifier: AGPL-3.0-only
//! `locusd` — the merge decision, over HTTP.
//!
//! The same evaluation as the CLI, out of the same [`gitlocus_core`] crate, so a
//! verdict from this service and a verdict from `locus verify` cannot disagree.
//! If they ever do, that is a bug here and not a property of the transport.
//!
//! **This service is a pure evaluator.** It stores nothing, reads no clock, and
//! makes no network call of its own — because a verdict is a pure function of
//! (policy, contribution, evidence) and keeping it that way is what lets anyone
//! recompute the answer offline and get the same one.
//!
//! Two consequences follow, and both are deliberate rather than deferred:
//!
//! - **No authentication.** There is nothing to protect: no state, no secrets,
//!   no side effects. Sending a contribution here reveals it to this server and
//!   nothing else, and the answer is one the sender could have computed
//!   themselves with the CLI. What remains is resource exhaustion, which is
//!   bounded by a request size limit rather than by a login.
//! - **No storage.** Nothing to leak, nothing to migrate, nothing to back up.
//!   When a ranked queue needs history it will need a store and an identity
//!   model, and that is a different service with a different threat model.

use gitlocus_core::policy::CompiledPolicy;
use gitlocus_core::{Contribution, Evidence, Policy, Verdict};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::SocketAddr;
use tiny_http::{Header, Request, Response, Server};

/// Largest request body accepted, in bytes.
///
/// Evaluation cost scales with the evidence array and the number of policy
/// rules, and this box is small. A cap is a cruder limit than metering the work
/// itself, and it is the one that cannot be got wrong.
const MAX_BODY: usize = 1024 * 1024;

/// The OpenAPI contract, served as the service's own description.
///
/// Kept as a file in the repository rather than generated, so it is reviewable
/// in a pull request and so a client can be written against it before the
/// endpoint exists.
const OPENAPI: &str = include_str!("../openapi.json");

/// What a caller sends to have a contribution evaluated.
#[derive(Debug, Deserialize)]
struct VerdictRequest {
    /// The policy at the revision under evaluation, as YAML.
    policy: String,
    /// The policy at the base revision, as YAML, where one exists.
    ///
    /// Sending it is what stops a contribution being judged by a document it
    /// wrote. Omitting it is legitimate only when the base revision genuinely
    /// has no policy — a first adoption — and the response says which happened.
    #[serde(default)]
    governing_policy: Option<String>,
    /// The change being evaluated.
    contribution: Contribution,
    /// What is known about it. `signer` is never read from here.
    #[serde(default)]
    evidence: Vec<Evidence>,
}

/// What the service returns.
#[derive(Debug, Serialize)]
struct VerdictResponse {
    verdict: Verdict,
    /// Whether a base-revision policy was supplied and applied.
    ///
    /// Reported because its absence changes what the verdict means, and a caller
    /// that forgot to send one should be able to notice from the answer rather
    /// than from the documentation.
    governed_by_base: bool,
}

/// Something the caller got wrong.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn json_response<T: Serialize>(status: u16, body: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    let encoded = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("a static header is always valid");
    Response::from_data(encoded)
        .with_status_code(status)
        .with_header(header)
}

fn error(status: u16, message: impl Into<String>) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        status,
        &ErrorResponse {
            error: message.into(),
        },
    )
}

/// Read a bounded body, refusing rather than truncating an oversized one.
///
/// Truncating would hand the evaluator a document that parses to something the
/// caller did not send, which is a worse failure than a rejection.
fn read_body(request: &mut Request) -> Result<Vec<u8>, String> {
    if let Some(length) = request.body_length()
        && length > MAX_BODY
    {
        return Err(format!("request body exceeds {MAX_BODY} bytes"));
    }
    let mut body = Vec::new();
    request
        .as_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|e| format!("reading request body: {e}"))?;
    if body.len() > MAX_BODY {
        return Err(format!("request body exceeds {MAX_BODY} bytes"));
    }
    Ok(body)
}

/// Compile a policy document, naming which one failed.
fn compile(source: &str, label: Option<&str>) -> Result<CompiledPolicy, String> {
    let parsed = Policy::from_yaml(source).map_err(|e| format!("parsing policy: {e}"))?;
    let parsed = match label {
        Some(label) => parsed.labelled(label),
        None => parsed,
    };
    parsed
        .compile()
        .map_err(|e| format!("compiling policy: {e}"))
}

fn evaluate(body: &[u8]) -> Result<VerdictResponse, String> {
    let request: VerdictRequest =
        serde_json::from_slice(body).map_err(|e| format!("parsing request: {e}"))?;

    // The governing policy is loaded first and its rules are labelled, so a
    // verdict says which document blocked the contribution.
    let mut policies = Vec::with_capacity(2);
    let governed_by_base = request.governing_policy.is_some();
    if let Some(base) = &request.governing_policy {
        policies.push(compile(base, Some("governing"))?);
    }
    policies.push(compile(&request.policy, None)?);

    let verdict =
        CompiledPolicy::merged(policies).evaluate(&request.contribution, &request.evidence);
    Ok(VerdictResponse {
        verdict,
        governed_by_base,
    })
}

fn handle(mut request: Request) {
    let method = request.method().as_str().to_owned();
    // Query strings are not used by any route, and matching on the whole URL
    // would silently 404 a caller who appended one.
    let path = request.url().split('?').next().unwrap_or("/").to_owned();

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/healthz") => json_response(
            200,
            &serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            }),
        ),
        ("GET", "/v0/openapi.json") => {
            let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                .expect("a static header is always valid");
            Response::from_data(OPENAPI.as_bytes().to_vec()).with_header(header)
        }
        ("POST", "/v0/verdict") => match read_body(&mut request) {
            Err(why) => error(413, why),
            Ok(body) => match evaluate(&body) {
                // A blocked verdict is a successful evaluation. The decision is
                // in the body; the status code describes the request, not the
                // contribution.
                Ok(response) => json_response(200, &response),
                Err(why) => error(400, why),
            },
        },
        ("GET" | "POST", _) => error(404, format!("no route for {method} {path}")),
        _ => error(405, format!("{method} is not supported")),
    };

    if let Err(e) = request.respond(response) {
        eprintln!("locusd: responding: {e}");
    }
}

fn main() {
    let addr: SocketAddr = std::env::var("LOCUSD_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("locusd: LOCUSD_ADDR is not a socket address: {e}");
            std::process::exit(2);
        });

    let server = Server::http(addr).unwrap_or_else(|e| {
        eprintln!("locusd: cannot listen on {addr}: {e}");
        std::process::exit(1);
    });
    println!("locusd {} listening on {addr}", env!("CARGO_PKG_VERSION"));

    for request in server.incoming_requests() {
        handle(request);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = "version: 0\nrules:\n  - name: baseline\n    when:\n      paths: [\"**\"]\n    require:\n      deterministic: [tests]\n";
    const CONTRIBUTION: &str = r#"{"repository":"github.com/acme/repo","base_digest":"aaaa","head_digest":"bbbb","actor":{"id":"someone","kind":"human","tier":"contributor"},"changed_paths":["src/main.rs"]}"#;

    fn request(evidence: &str, governing: Option<&str>) -> Vec<u8> {
        let governing = governing.map_or("null".to_string(), |g| {
            serde_json::to_string(g).expect("a string always encodes")
        });
        format!(
            r#"{{"policy":{},"governing_policy":{governing},"contribution":{CONTRIBUTION},"evidence":{evidence}}}"#,
            serde_json::to_string(POLICY).unwrap()
        )
        .into_bytes()
    }

    #[test]
    fn a_contribution_with_its_evidence_is_satisfied() {
        let evidence = r#"[{"kind":"tests","class":"deterministic","outcome":"pass","subject_digest":"bbbb","produced_by":"ci","produced_at":"2026-08-19T00:00:00Z"}]"#;
        let response = evaluate(&request(evidence, None)).expect("valid request");
        assert_eq!(
            response.verdict.decision,
            gitlocus_core::Decision::Satisfied
        );
        assert!(!response.governed_by_base);
    }

    #[test]
    fn a_contribution_without_its_evidence_is_blocked() {
        let response = evaluate(&request("[]", None)).expect("valid request");
        assert_eq!(response.verdict.decision, gitlocus_core::Decision::Blocked);
    }

    #[test]
    fn a_signer_sent_in_the_request_is_discarded() {
        // The whole value of a signed_by rule rests on this, and a network
        // boundary is exactly where someone would try it: the field is
        // skip_deserializing in the core, and this asserts the service inherits
        // that rather than reintroducing it in its own request type.
        let forged = r#"[{"kind":"tests","class":"deterministic","outcome":"pass","subject_digest":"bbbb","produced_by":"me","produced_at":"2026-08-19T00:00:00Z","signer":"https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"}]"#;
        let response = evaluate(&request(forged, None)).expect("valid request");
        let encoded = serde_json::to_string(&response).expect("verdict encodes");
        assert!(
            !encoded.contains("workflows/ci.yml"),
            "a signer must never survive the request boundary: {encoded}"
        );
    }

    #[test]
    fn a_rule_deleted_by_the_contribution_still_governs_it() {
        // ADR 0013 across the network boundary. The gutted policy alone is
        // satisfied; with the base policy supplied it is not.
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

    #[test]
    fn the_openapi_document_is_valid_json_and_describes_the_routes() {
        let doc: serde_json::Value =
            serde_json::from_str(OPENAPI).expect("the contract must be valid JSON");
        let paths = doc.get("paths").expect("a contract describes paths");
        for route in ["/healthz", "/v0/verdict", "/v0/openapi.json"] {
            assert!(paths.get(route).is_some(), "{route} is not described");
        }
    }
}
