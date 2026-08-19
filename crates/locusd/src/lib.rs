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
//!
//! Routing and the body limit live here rather than in the binary because they
//! are the parts where a mistake is *silent* — a deleted route answers 404 and a
//! broken limit answers 200 — so they are the parts that most need to be
//! testable without a socket.

use gitlocus_core::policy::CompiledPolicy;
use gitlocus_core::{Contribution, Evidence, Policy, Verdict};
use serde::{Deserialize, Serialize};
use std::io::Read;
use tiny_http::{Header, Request, Response, Server};

/// Largest request body accepted, in bytes.
///
/// Evaluation cost scales with the evidence array and the number of policy
/// rules, and this box is small. A cap is a cruder limit than metering the work
/// itself, and it is the one that cannot be got wrong.
pub const MAX_BODY: usize = 1024 * 1024;

/// The OpenAPI contract, served as the service's own description.
///
/// Kept as a file in the repository rather than generated, so it is reviewable
/// in a pull request and so a client can be written against it before the
/// endpoint exists.
pub const OPENAPI: &str = include_str!("../openapi.json");

/// What a caller sends to have a contribution evaluated.
#[derive(Debug, Deserialize)]
pub struct VerdictRequest {
    /// The policy at the revision under evaluation, as YAML.
    pub policy: String,
    /// The policy at the base revision, as YAML, where one exists.
    ///
    /// Sending it is what stops a contribution being judged by a document it
    /// wrote. Omitting it is legitimate only when the base revision genuinely
    /// has no policy — a first adoption — and the response says which happened.
    #[serde(default)]
    pub governing_policy: Option<String>,
    /// The change being evaluated.
    pub contribution: Contribution,
    /// What is known about it. `signer` is never read from here.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

/// What the service returns.
#[derive(Debug, Serialize)]
pub struct VerdictResponse {
    /// The verdict itself.
    pub verdict: Verdict,
    /// Whether a base-revision policy was supplied and applied.
    ///
    /// Reported because its absence changes what the verdict means, and a caller
    /// that forgot to send one should be able to notice from the answer rather
    /// than from the documentation.
    pub governed_by_base: bool,
}

/// Something the caller got wrong.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// What was wrong with the request.
    pub error: String,
}

/// A status and a body. No headers: every route answers JSON.
#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    /// HTTP status.
    pub status: u16,
    /// JSON body.
    pub body: Vec<u8>,
}

fn encode<T: Serialize>(status: u16, body: &T) -> Reply {
    Reply {
        status,
        body: serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec()),
    }
}

fn client_error(status: u16, message: impl Into<String>) -> Reply {
    encode(
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
///
/// A declared length is checked first so an oversized body can be refused
/// without reading it, and the read is capped regardless, because the declared
/// length is the caller's claim about the caller's own request.
///
/// # Errors
/// Returns a message when the body is too large or cannot be read.
pub fn read_bounded(declared: Option<usize>, reader: &mut impl Read) -> Result<Vec<u8>, String> {
    let too_big = || format!("request body exceeds {MAX_BODY} bytes");
    if declared.is_some_and(|length| length > MAX_BODY) {
        return Err(too_big());
    }
    let mut body = Vec::new();
    reader
        .take(MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|e| format!("reading request body: {e}"))?;
    if body.len() > MAX_BODY {
        return Err(too_big());
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

/// Evaluate a request body into a verdict.
///
/// # Errors
/// Returns a message when the request, policy or contribution cannot be read.
pub fn evaluate(body: &[u8]) -> Result<VerdictResponse, String> {
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

/// Answer one request. Pure: no socket, no clock, no state.
#[must_use]
pub fn route(method: &str, url: &str, body: &[u8]) -> Reply {
    // Query strings are not used by any route, and matching on the whole URL
    // would silently 404 a caller who appended one.
    let path = url.split('?').next().unwrap_or("/");

    match (method, path) {
        ("GET", "/healthz") => encode(
            200,
            &serde_json::json!({
                "status": "ok",
                "version": env!("CARGO_PKG_VERSION"),
            }),
        ),
        ("GET", "/v0/openapi.json") => Reply {
            status: 200,
            body: OPENAPI.as_bytes().to_vec(),
        },
        // A blocked verdict is a successful evaluation. The decision is in the
        // body; the status code describes the request, not the contribution.
        ("POST", "/v0/verdict") => match evaluate(body) {
            Ok(response) => encode(200, &response),
            Err(why) => client_error(400, why),
        },
        ("GET" | "POST", _) => client_error(404, format!("no route for {method} {path}")),
        _ => client_error(405, format!("{method} is not supported")),
    }
}

fn handle(mut request: Request) {
    let method = request.method().as_str().to_owned();
    let url = request.url().to_owned();

    let reply = if method == "POST" {
        let declared = request.body_length();
        match read_bounded(declared, &mut request.as_reader()) {
            Ok(body) => route(&method, &url, &body),
            Err(why) => client_error(413, why),
        }
    } else {
        route(&method, &url, &[])
    };

    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("a static header is always valid");
    let response = Response::from_data(reply.body)
        .with_status_code(reply.status)
        .with_header(header);
    if let Err(e) = request.respond(response) {
        eprintln!("locusd: responding: {e}");
    }
}

/// Serve until the listener stops.
///
/// Exposed so a test can drive a real socket rather than only the pure routing.
pub fn serve(server: &Server) {
    for request in server.incoming_requests() {
        handle(request);
    }
}
