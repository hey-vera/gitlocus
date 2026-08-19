// SPDX-License-Identifier: Apache-2.0
//! The repository's own rules.
//!
//! GitLocus sets no bar of its own. A policy lives in the repository it governs,
//! is versioned with it, and is evaluated deterministically — the same policy,
//! contribution and evidence always produce the same verdict, whether that
//! evaluation happens on a laptop or in CI.

use crate::actor::TrustTier;
use crate::authorship::{self, AuthorshipClaim, AuthorshipKind};
use crate::contribution::Contribution;
use crate::evidence::{Evidence, EvidenceClass, Outcome};
use crate::verdict::{Decision, Rank, Unmet, UnmetReason, Verdict};
use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Schema version of the policy document.
pub const POLICY_VERSION: u32 = 0;

/// Something wrong with a policy document.
#[derive(Debug)]
pub enum PolicyError {
    /// The document declares a schema version this build does not understand.
    UnsupportedVersion {
        /// The version the document declared.
        found: u32,
    },
    /// A path pattern failed to parse.
    BadGlob {
        /// The rule the pattern came from.
        rule: String,
        /// The offending pattern.
        pattern: String,
    },
    /// The document could not be parsed at all.
    Malformed(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => {
                write!(
                    f,
                    "unsupported policy version {found}, expected {POLICY_VERSION}"
                )
            }
            Self::BadGlob { rule, pattern } => {
                write!(f, "rule {rule} has an invalid path pattern {pattern}")
            }
            Self::Malformed(why) => write!(f, "malformed policy: {why}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Conditions under which a rule applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct When {
    /// Glob patterns matched against the contribution's changed paths.
    pub paths: Vec<String>,
}

/// What a rule demands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Require {
    /// Named checks that must be present, deterministic, and passing.
    #[serde(default)]
    pub deterministic: Vec<String>,
    /// How many human attestations are needed.
    #[serde(default)]
    pub approvals: u32,
    /// Minimum standing the contributor must hold.
    #[serde(default = "min_tier_default")]
    pub min_tier: TrustTier,
    /// Checks that must additionally carry a verified signature, and from whom.
    ///
    /// The key is a requirement name; the value is a glob matched against the
    /// verified signer identity.
    ///
    /// **Pin the workflow path, not the issuer.** Anyone can run a workflow in
    /// their own fork and get a valid signing identity from the same issuer, so
    /// a glob of `*` — or one matching only `token.actions.githubusercontent.com`
    /// — accepts a result produced by an arbitrary party running arbitrary code.
    /// That is the exact situation this field exists to prevent.
    ///
    /// ```yaml
    /// signed_by:
    ///   tests: "https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"
    ///   lint: "*"
    /// ```
    ///
    /// This is what stops a passing result produced on a laptop from standing in
    /// for one produced by the repository's own CI. Without it, evidence is only
    /// as trustworthy as the least careful party who can write the file.
    #[serde(default)]
    pub signed_by: BTreeMap<String, String>,

    /// Identity an attestation must be signed by in order to count as an approval.
    ///
    /// Without this, an approval is a record whose `produced_by` is a string the
    /// producer chose. An agent that has been talked into doing an attacker's
    /// bidding — by an issue body, a README, a dependency's documentation — can
    /// emit one and approve its own work. Class separation does not help here:
    /// the record really is `attested`, it is simply nobody's attestation.
    ///
    /// Pointing this at a human identity provider is what makes the blast radius
    /// of a hijacked agent "an unwanted contribution was opened" rather than
    /// "an unwanted contribution was merged".
    ///
    /// ```yaml
    /// approvals: 1
    /// approvals_signed_by: "https://github.com/login/oauth/*"
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approvals_signed_by: Option<String>,

    /// Authorship claims this rule accepts.
    ///
    /// Empty says nothing about authorship and constrains nothing. Naming any
    /// claim constrains all of them: what is not named is refused.
    ///
    /// ```yaml
    /// require:
    ///   authorship: [human, directed_agent]   # generated code cannot enter
    /// ```
    ///
    /// **Silence is read as `generated`.** A contribution that declares nothing
    /// fails a rule that does not accept `generated`, which is what makes
    /// asserting authorship a deliberate act rather than a default nobody
    /// noticed. See [`crate::authorship`].
    ///
    /// Where several matching rules constrain authorship, a claim must be
    /// accepted by **all** of them — the intersection, not the union. Two rules
    /// saying different things about one contribution is stricter, not
    /// ambiguous, exactly as it is for signer globs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authorship: Vec<AuthorshipKind>,
}

fn min_tier_default() -> TrustTier {
    TrustTier::Unknown
}

/// One rule: when it applies, and what it demands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name, reported in the verdict.
    pub name: String,
    /// When the rule applies.
    pub when: When,
    /// What it demands.
    pub require: Require,
}

/// A policy document, as written in `.gitlocus/policy.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Schema version. Must equal [`POLICY_VERSION`].
    pub version: u32,
    /// Rules, evaluated together. Every matching rule contributes.
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Parse a policy from YAML.
    ///
    /// # Errors
    /// Returns [`PolicyError::Malformed`] if the document is not valid YAML in
    /// the expected shape.
    pub fn from_yaml(src: &str) -> Result<Self, PolicyError> {
        serde_norway::from_str(src).map_err(|e| PolicyError::Malformed(e.to_string()))
    }

    /// Prefix every rule name, so a verdict says which document a rule came from.
    ///
    /// Without this, a contribution blocked by a rule it deleted reports the name
    /// of a rule the reader cannot find in the file in front of them. `governing:
    /// ci-and-policy` says the necessary thing: the rule you removed still applies
    /// to the change that removed it.
    #[must_use]
    pub fn labelled(mut self, label: &str) -> Self {
        for rule in &mut self.rules {
            rule.name = format!("{label}:{}", rule.name);
        }
        self
    }

    /// Compile path patterns so that evaluation cannot fail.
    ///
    /// # Errors
    /// Returns [`PolicyError::UnsupportedVersion`] or [`PolicyError::BadGlob`].
    pub fn compile(self) -> Result<CompiledPolicy, PolicyError> {
        if self.version != POLICY_VERSION {
            return Err(PolicyError::UnsupportedVersion {
                found: self.version,
            });
        }
        let mut compiled = Vec::with_capacity(self.rules.len());
        for rule in self.rules {
            let mut builder = GlobSetBuilder::new();
            for pattern in &rule.when.paths {
                let glob = Glob::new(pattern).map_err(|_| PolicyError::BadGlob {
                    rule: rule.name.clone(),
                    pattern: pattern.clone(),
                })?;
                builder.add(glob);
            }
            let set = builder
                .build()
                .map_err(|e| PolicyError::Malformed(e.to_string()))?;

            // Signer constraints are globs over an identity URI, compiled here
            // for the same reason path globs are: evaluation must not be able
            // to fail partway through and leave a half-formed verdict.
            let mut signers = BTreeMap::new();
            for (kind, pattern) in &rule.require.signed_by {
                let glob = Glob::new(pattern).map_err(|_| PolicyError::BadGlob {
                    rule: rule.name.clone(),
                    pattern: pattern.clone(),
                })?;
                signers.insert(kind.clone(), glob.compile_matcher());
            }

            let approvals_signer = match &rule.require.approvals_signed_by {
                Some(pattern) => Some(
                    Glob::new(pattern)
                        .map_err(|_| PolicyError::BadGlob {
                            rule: rule.name.clone(),
                            pattern: pattern.clone(),
                        })?
                        .compile_matcher(),
                ),
                None => None,
            };

            compiled.push(CompiledRule {
                rule,
                paths: set,
                signers,
                approvals_signer,
            });
        }
        Ok(CompiledPolicy { rules: compiled })
    }
}

/// A policy with its path patterns compiled. Evaluation is infallible.
#[derive(Debug)]
pub struct CompiledPolicy {
    rules: Vec<CompiledRule>,
}

/// A rule with its path and signer globs compiled.
#[derive(Debug)]
struct CompiledRule {
    rule: Rule,
    paths: GlobSet,
    signers: BTreeMap<String, GlobMatcher>,
    approvals_signer: Option<GlobMatcher>,
}

/// Everything the matching rules ask of a contribution, combined.
///
/// Separated from evaluation so that "what did the policy ask for" and "does the
/// evidence answer it" stay legible as two questions rather than one long one.
struct Demands<'a> {
    matched: Vec<String>,
    required: BTreeSet<&'a str>,
    approvals_required: u32,
    tier_required: TrustTier,
    signer_constraints: Vec<(&'a str, &'a GlobMatcher)>,
    approval_signers: Vec<&'a GlobMatcher>,
    authorship_accepted: Option<BTreeSet<AuthorshipKind>>,
}

impl CompiledPolicy {
    /// Combine every matching rule into one set of demands.
    ///
    /// Requirements union, approvals and tier take the strictest, every signer
    /// glob applies, and accepted authorship claims **intersect** — a claim has
    /// to be acceptable to all of them. Union anywhere here would let a
    /// contribution shop for the most permissive rule it happened to match.
    fn demands_of(&self, contribution: &Contribution) -> Demands<'_> {
        let mut d = Demands {
            matched: Vec::new(),
            required: BTreeSet::new(),
            approvals_required: 0,
            tier_required: TrustTier::Unknown,
            signer_constraints: Vec::new(),
            approval_signers: Vec::new(),
            authorship_accepted: None,
        };

        for compiled in &self.rules {
            if !contribution
                .changed_paths
                .iter()
                .any(|p| compiled.paths.is_match(p))
            {
                continue;
            }
            let rule = &compiled.rule;
            d.matched.push(rule.name.clone());
            d.required
                .extend(rule.require.deterministic.iter().map(String::as_str));
            d.approvals_required = d.approvals_required.max(rule.require.approvals);
            d.tier_required = d.tier_required.max(rule.require.min_tier);

            if let Some(matcher) = &compiled.approvals_signer {
                d.approval_signers.push(matcher);
            }

            if !rule.require.authorship.is_empty() {
                let named: BTreeSet<AuthorshipKind> =
                    rule.require.authorship.iter().copied().collect();
                d.authorship_accepted = Some(match d.authorship_accepted.take() {
                    Some(existing) => existing.intersection(&named).copied().collect(),
                    None => named,
                });
            }

            for (kind, matcher) in &compiled.signers {
                // A signature requirement implies the check is required at all.
                // Demanding a signature on something optional would silently do
                // nothing, which is worse than either alternative.
                d.required.insert(kind.as_str());
                d.signer_constraints.push((kind.as_str(), matcher));
            }
        }
        d
    }

    /// Combine policies so that every rule in every one of them applies.
    ///
    /// A contribution is governed by the policy at the revision under evaluation
    /// **and** by the policy at the revision it is proposed against. Without the
    /// second, a contribution can delete the rule that would have blocked it and
    /// be judged by what remains — honest evaluation of a document the
    /// contributor wrote, reaching a conclusion the repository never agreed to.
    ///
    /// The combination needs no new semantics. Evaluation already unions required
    /// checks, takes the strictest approvals and tier, and demands that every
    /// signer glob constraining a check matches, so concatenating the rules of
    /// two documents produces exactly the intended reading and the result is
    /// **never weaker than any input**.
    ///
    /// The asymmetry this creates is the desirable one: a rule a contribution
    /// adds binds that contribution immediately, and a rule it removes keeps
    /// binding until the change removing it has itself been accepted.
    #[must_use]
    pub fn merged(policies: Vec<Self>) -> Self {
        Self {
            rules: policies.into_iter().flat_map(|p| p.rules).collect(),
        }
    }

    /// Evaluate a contribution against this policy.
    ///
    /// Every rule whose paths match contributes: required checks are unioned, and
    /// approvals and minimum tier take the strictest value demanded. A change that
    /// touches both ordinary source and CI configuration is therefore held to the
    /// CI rule, which is the only safe way to combine the two.
    #[must_use]
    pub fn evaluate(&self, contribution: &Contribution, evidence: &[Evidence]) -> Verdict {
        let digest = contribution.head_digest.as_str();
        let Demands {
            matched,
            required,
            approvals_required,
            tier_required,
            signer_constraints,
            approval_signers,
            authorship_accepted,
        } = self.demands_of(contribution);

        let mut unmet = Vec::new();
        let mut satisfied = 0_u32;
        for kind in &required {
            let signers: Vec<&GlobMatcher> = signer_constraints
                .iter()
                .filter(|(k, _)| k == kind)
                .map(|(_, m)| *m)
                .collect();
            match classify(kind, digest, evidence, &signers) {
                None => satisfied += 1,
                Some(reason) => unmet.push(Unmet {
                    requirement: (*kind).to_string(),
                    reason,
                }),
            }
        }

        // Authorship is a requirement like any other, so it counts toward the
        // total and toward confidence rather than sitting outside the ranking.
        let mut total = u32::try_from(required.len()).unwrap_or(u32::MAX);
        if let Some(accepted) = &authorship_accepted {
            total = total.saturating_add(1);
            match classify_authorship(digest, evidence, accepted) {
                None => satisfied += 1,
                Some(reason) => unmet.push(Unmet {
                    requirement: "authorship".to_string(),
                    reason,
                }),
            }
        }

        // An approval is counted by verified signer where the policy demands
        // one, and by the self-asserted producer otherwise. Distinct approvals
        // are counted, so one party attesting twice is still one approval.
        let approvals_present = u32::try_from(
            evidence
                .iter()
                .filter(|e| {
                    e.class == EvidenceClass::Attested
                        && e.outcome == Outcome::Pass
                        && e.subject_digest == digest
                })
                .filter_map(|e| {
                    if approval_signers.is_empty() {
                        return Some(e.produced_by.as_str());
                    }
                    e.signer
                        .as_deref()
                        .filter(|s| approval_signers.iter().all(|m| m.is_match(s)))
                })
                .collect::<BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX);

        let advisory = evidence
            .iter()
            .filter(|e| e.class == EvidenceClass::Assessed && e.subject_digest == digest)
            .map(|e| format!("{}: {:?}", e.kind, e.outcome))
            .collect();

        let tier_satisfied = contribution.actor.tier.satisfies(tier_required);

        let decision = if !tier_satisfied || !unmet.is_empty() {
            Decision::Blocked
        } else if approvals_present < approvals_required {
            Decision::NeedsHuman
        } else {
            Decision::Satisfied
        };

        let confidence = if total == 0 {
            1.0
        } else {
            f64::from(satisfied) / f64::from(total)
        };

        Verdict {
            contribution: contribution.id(),
            subject_digest: digest.to_string(),
            decision,
            matched_rules: matched,
            unmet,
            approvals_required,
            approvals_present,
            tier_required,
            tier_satisfied,
            advisory,
            rank: Rank {
                confidence,
                human_cost: approvals_required.saturating_sub(approvals_present),
            },
        }
    }
}

/// Whether the authorship a contribution declares is one the policy accepts.
///
/// Only `attested` records are consulted. A declaration is a party accepting
/// responsibility for a statement, and honouring one on a `deterministic` or
/// `assessed` record would let a test runner or a model declare authorship —
/// which is precisely what this mechanism exists to make impossible.
///
/// **Every declaration must be accepted, not merely one of them.** A
/// contribution carrying both a `human` and a `generated` claim contains
/// generated work, and a policy refusing generated work refuses it.
fn classify_authorship(
    digest: &str,
    evidence: &[Evidence],
    accepted: &BTreeSet<AuthorshipKind>,
) -> Option<UnmetReason> {
    let declared: BTreeSet<AuthorshipKind> = evidence
        .iter()
        .filter(|e| {
            e.class == EvidenceClass::Attested
                && e.outcome == Outcome::Pass
                && e.subject_digest == digest
        })
        .filter_map(|e| e.authorship.as_ref().map(AuthorshipClaim::kind))
        .collect();

    if declared.is_empty() {
        // Silence is not a claim: the weakest applies.
        return if accepted.contains(&authorship::UNDECLARED) {
            None
        } else {
            Some(UnmetReason::Undeclared)
        };
    }

    if declared.is_subset(accepted) {
        None
    } else {
        Some(UnmetReason::WrongAuthorship)
    }
}

/// Why a requirement is unmet, or `None` if it is met.
///
/// `signers` carries every signer constraint the matching rules placed on this
/// requirement. All of them must match the same piece of evidence.
fn classify(
    kind: &str,
    digest: &str,
    evidence: &[Evidence],
    signers: &[&GlobMatcher],
) -> Option<UnmetReason> {
    let candidates: Vec<&Evidence> = evidence.iter().filter(|e| e.kind == kind).collect();
    if candidates.is_empty() {
        return Some(UnmetReason::Missing);
    }

    let binding: Vec<&&Evidence> = candidates
        .iter()
        .filter(|e| e.is_binding_for(digest))
        .collect();

    if !binding.is_empty() {
        if signers.is_empty() {
            return None;
        }
        // Signature is checked only against evidence that would otherwise bind,
        // so a failing signed record cannot mask a passing unsigned one or the
        // other way round.
        if binding.iter().any(|e| {
            e.signer
                .as_deref()
                .is_some_and(|s| signers.iter().all(|m| m.is_match(s)))
        }) {
            return None;
        }
        return Some(if binding.iter().any(|e| e.signer.is_some()) {
            UnmetReason::WrongSigner
        } else {
            UnmetReason::Unsigned
        });
    }
    // Report the most actionable reason rather than whichever was seen first.
    if candidates
        .iter()
        .any(|e| e.outcome == Outcome::Fail && e.subject_digest == digest)
    {
        return Some(UnmetReason::Failed);
    }
    if candidates
        .iter()
        .any(|e| e.outcome == Outcome::Inconclusive && e.subject_digest == digest)
    {
        return Some(UnmetReason::Inconclusive);
    }
    if candidates.iter().all(|e| e.subject_digest != digest) {
        return Some(UnmetReason::StaleSubject);
    }
    Some(UnmetReason::NotDeterministic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Actor, ActorKind};

    const POLICY: &str = r#"
version: 0
rules:
  - name: baseline
    when:
      paths: ["**"]
    require:
      deterministic: [build, tests]
      approvals: 1
      min_tier: unknown
  - name: ci-config
    when:
      paths: [".github/workflows/**"]
    require:
      deterministic: [workflow-audit]
      approvals: 1
      min_tier: maintainer
"#;

    fn policy() -> CompiledPolicy {
        Policy::from_yaml(POLICY).unwrap().compile().unwrap()
    }

    fn contribution(paths: &[&str], tier: TrustTier) -> Contribution {
        Contribution {
            repository: "github.com/hey-vera/gitlocus".into(),
            base_digest: "base".into(),
            head_digest: "head".into(),
            actor: Actor {
                id: "someone".into(),
                kind: ActorKind::Human,
                tier,
                key_binding: None,
            },
            changed_paths: paths.iter().map(|s| (*s).to_string()).collect(),
            forge_ref: None,
        }
    }

    fn pass(kind: &str) -> Evidence {
        Evidence {
            kind: kind.into(),
            class: EvidenceClass::Deterministic,
            outcome: Outcome::Pass,
            subject_digest: "head".into(),
            produced_by: "ci".into(),
            produced_at: "2026-08-18T00:00:00Z".into(),
            source_uri: None,
            summary: None,
            authorship: None,
            signer: None,
        }
    }

    fn approval(by: &str) -> Evidence {
        Evidence {
            kind: "review".into(),
            class: EvidenceClass::Attested,
            outcome: Outcome::Pass,
            subject_digest: "head".into(),
            produced_by: by.into(),
            produced_at: "2026-08-18T00:00:00Z".into(),
            source_uri: None,
            summary: None,
            authorship: None,
            signer: None,
        }
    }

    #[test]
    fn missing_evidence_blocks() {
        let v = policy().evaluate(&contribution(&["src/main.rs"], TrustTier::Contributor), &[]);
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet.len(), 2);
    }

    #[test]
    fn all_checks_pass_but_approval_outstanding_needs_a_human() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests")],
        );
        assert_eq!(v.decision, Decision::NeedsHuman);
        assert_eq!(v.rank.human_cost, 1);
        assert!((v.rank.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn checks_plus_approval_satisfies() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests"), approval("maintainer-a")],
        );
        assert_eq!(v.decision, Decision::Satisfied);
        assert_eq!(v.rank.human_cost, 0);
    }

    #[test]
    fn duplicate_approvals_from_one_actor_count_once() {
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[
                pass("build"),
                pass("tests"),
                approval("same"),
                approval("same"),
            ],
        );
        assert_eq!(v.approvals_present, 1);
    }

    #[test]
    fn touching_ci_config_pulls_in_the_stricter_rule() {
        let c = contribution(
            &["src/main.rs", ".github/workflows/ci.yml"],
            TrustTier::Contributor,
        );
        let v = policy().evaluate(&c, &[pass("build"), pass("tests"), pass("workflow-audit")]);
        // Requirements unioned across both matched rules.
        assert_eq!(v.matched_rules.len(), 2);
        // Contributor does not clear the maintainer bar the CI rule demands.
        assert!(!v.tier_satisfied);
        assert_eq!(v.decision, Decision::Blocked);
    }

    #[test]
    fn assessed_evidence_never_satisfies_a_requirement() {
        let mut ai = pass("tests");
        ai.class = EvidenceClass::Assessed;
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), ai],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::NotDeterministic);
        // ...but it is still surfaced, because it may well be worth reading.
        assert_eq!(v.advisory.len(), 1);
    }

    #[test]
    fn evidence_for_a_previous_revision_is_reported_as_stale() {
        let mut old = pass("tests");
        old.subject_digest = "an-older-revision".into();
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), old],
        );
        assert_eq!(v.unmet[0].reason, UnmetReason::StaleSubject);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);
        let ev = [pass("build"), pass("tests")];
        let a = policy().evaluate(&c, &ev);
        let b = policy().evaluate(&c, &ev);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn a_policy_error_says_what_is_wrong_and_where() {
        // The only thing a contributor with a broken policy ever sees. An error
        // that renders to nothing turns "your glob is malformed" into silence,
        // and there is no other diagnostic behind it.
        let version = PolicyError::UnsupportedVersion { found: 99 }.to_string();
        assert!(
            version.contains("99"),
            "must name the version found: {version}"
        );
        assert!(version.contains('0'), "and the one expected: {version}");

        let glob = PolicyError::BadGlob {
            rule: "ci-and-policy".into(),
            pattern: "[".into(),
        }
        .to_string();
        assert!(glob.contains("ci-and-policy"), "must name the rule: {glob}");
        assert!(glob.contains('['), "and the pattern: {glob}");

        let malformed = PolicyError::Malformed("expected a mapping".into()).to_string();
        assert!(
            malformed.contains("expected a mapping"),
            "must carry the underlying reason: {malformed}"
        );
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let src = "version: 99\nrules: []\n";
        let err = Policy::from_yaml(src).unwrap().compile().unwrap_err();
        assert!(matches!(err, PolicyError::UnsupportedVersion { found: 99 }));
    }

    // --- signature constraints -------------------------------------------

    const SIGNED_POLICY: &str = r#"
version: 0
rules:
  - name: signed-ci
    when:
      paths: ["**"]
    require:
      deterministic: [tests]
      approvals: 0
      signed_by:
        tests: "https://github.com/acme/repo/.github/workflows/ci.yml@*"
"#;

    fn signed_policy() -> CompiledPolicy {
        Policy::from_yaml(SIGNED_POLICY).unwrap().compile().unwrap()
    }

    fn signed(kind: &str, by: Option<&str>) -> Evidence {
        let mut e = pass(kind);
        e.signer = by.map(ToOwned::to_owned);
        e
    }

    #[test]
    fn unsigned_evidence_does_not_satisfy_a_signed_requirement() {
        let v = signed_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[signed("tests", None)],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::Unsigned);
    }

    #[test]
    fn evidence_signed_by_the_wrong_identity_does_not_satisfy_it() {
        // The laptop case: a real passing run, really signed, by someone who is
        // not the repository's CI.
        let v = signed_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[signed(
                "tests",
                Some(
                    "https://github.com/someone-else/repo/.github/workflows/ci.yml@refs/heads/main",
                ),
            )],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::WrongSigner);
    }

    #[test]
    fn evidence_signed_by_the_expected_identity_satisfies_it() {
        let v = signed_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[signed(
                "tests",
                Some("https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"),
            )],
        );
        assert_eq!(v.decision, Decision::Satisfied, "unmet: {:?}", v.unmet);
    }

    #[test]
    fn a_signature_requirement_makes_the_check_required() {
        // Demanding a signature on a check nobody required would silently do
        // nothing, which is a worse outcome than either alternative.
        let v =
            signed_policy().evaluate(&contribution(&["src/main.rs"], TrustTier::Contributor), &[]);
        assert_eq!(v.unmet[0].requirement, "tests");
        assert_eq!(v.unmet[0].reason, UnmetReason::Missing);
    }

    #[test]
    fn a_wildcard_still_demands_that_somebody_signed() {
        let src = r#"
version: 0
rules:
  - name: any-signer
    when:
      paths: ["**"]
    require:
      deterministic: [tests]
      approvals: 0
      signed_by:
        tests: "*"
"#;
        let policy = || Policy::from_yaml(src).unwrap().compile().unwrap();
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);

        let unsigned = policy().evaluate(&c, &[signed("tests", None)]);
        assert_eq!(unsigned.unmet[0].reason, UnmetReason::Unsigned);

        let anyone = policy().evaluate(&c, &[signed("tests", Some("https://example.com/whoever"))]);
        assert_eq!(anyone.decision, Decision::Satisfied);
    }

    #[test]
    fn a_signed_failure_cannot_be_masked_by_an_unsigned_pass() {
        // Two records for one check: one correctly signed but failing, one
        // passing with no signature. Neither should satisfy the requirement.
        let mut failing = signed(
            "tests",
            Some("https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"),
        );
        failing.outcome = Outcome::Fail;

        let v = signed_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[failing, signed("tests", None)],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::Unsigned);
    }

    #[test]
    fn a_forged_signer_in_input_json_cannot_satisfy_a_signed_requirement() {
        // The end-to-end version of the guarantee in evidence.rs: write the
        // trusted CI identity into an evidence file by hand and it still fails.
        let forged = format!(
            r#"[{{"kind":"tests","class":"deterministic","outcome":"pass",
                 "subject_digest":"head","produced_by":"me",
                 "produced_at":"2026-08-18T00:00:00Z",
                 "signer":"{}"}}]"#,
            "https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"
        );
        let evidence: Vec<Evidence> = serde_json::from_str(&forged).unwrap();

        let v = signed_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &evidence,
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::Unsigned);
    }

    #[test]
    fn signature_constraints_do_not_disturb_determinism() {
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);
        let ev = [
            signed(
                "tests",
                Some("https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"),
            ),
            signed("tests", None),
        ];
        let mut reversed = ev.clone();
        reversed.reverse();
        assert_eq!(
            serde_json::to_string(&signed_policy().evaluate(&c, &ev)).unwrap(),
            serde_json::to_string(&signed_policy().evaluate(&c, &reversed)).unwrap()
        );
    }

    // --- approvals must come from someone ---------------------------------

    const HUMAN_APPROVAL: &str = r#"
version: 0
rules:
  - name: human-approval
    when:
      paths: ["**"]
    require:
      approvals: 1
      approvals_signed_by: "https://github.com/login/oauth/*"
"#;

    fn human_policy() -> CompiledPolicy {
        Policy::from_yaml(HUMAN_APPROVAL)
            .unwrap()
            .compile()
            .unwrap()
    }

    fn attested(by: &str, signer: Option<&str>) -> Evidence {
        let mut e = approval(by);
        e.signer = signer.map(ToOwned::to_owned);
        e
    }

    #[test]
    fn an_agent_cannot_manufacture_its_own_approval() {
        // The hijacked-agent case. The record really is attested; it is simply
        // nobody's attestation, because produced_by is a string the producer
        // chose. Class separation does not help here — this does.
        let v = human_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[attested("definitely-a-human", None)],
        );
        assert_eq!(v.decision, Decision::NeedsHuman);
        assert_eq!(v.approvals_present, 0);
    }

    #[test]
    fn an_approval_signed_by_a_workflow_does_not_count_as_human() {
        // A CI identity is not a person, and a policy asking for human sign-off
        // should not accept one however genuine the signature.
        let v = human_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[attested(
                "ci",
                Some("https://github.com/acme/repo/.github/workflows/ci.yml@refs/heads/main"),
            )],
        );
        assert_eq!(v.approvals_present, 0);
        assert_eq!(v.decision, Decision::NeedsHuman);
    }

    #[test]
    fn an_approval_signed_by_a_human_identity_counts() {
        let v = human_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[attested(
                "josh",
                Some("https://github.com/login/oauth/josh"),
            )],
        );
        assert_eq!(v.approvals_present, 1);
        assert_eq!(v.decision, Decision::Satisfied);
    }

    #[test]
    fn signed_approvals_are_counted_by_signer_not_by_claimed_producer() {
        // One human, two records claiming different producers. Still one
        // approval: otherwise a single party could satisfy a two-approval rule
        // by varying a string.
        let v = human_policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[
                attested("alice", Some("https://github.com/login/oauth/josh")),
                attested("bob", Some("https://github.com/login/oauth/josh")),
            ],
        );
        assert_eq!(v.approvals_present, 1);
    }

    // --- authorship ---------------------------------------------------------

    const HUMAN_ONLY: &str = r#"
version: 0
rules:
  - name: licence-integrity
    when:
      paths: ["**"]
    require:
      authorship: [human, directed_agent]
"#;

    fn human_authorship() -> CompiledPolicy {
        Policy::from_yaml(HUMAN_ONLY).unwrap().compile().unwrap()
    }

    fn declares(claim: AuthorshipClaim) -> Evidence {
        let mut e = approval("a-named-human");
        e.kind = "authorship".into();
        e.authorship = Some(claim);
        e
    }

    #[test]
    fn silence_is_read_as_generated_and_refused() {
        // The property the whole mechanism rests on. If undeclared work passed a
        // human-only rule, every unlabelled contribution would quietly claim
        // copyright and the licence would dilute exactly as before.
        let v = human_authorship().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build")],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].requirement, "authorship");
        assert_eq!(v.unmet[0].reason, UnmetReason::Undeclared);
    }

    #[test]
    fn a_declaration_the_policy_accepts_satisfies_it() {
        for claim in [AuthorshipClaim::Human, AuthorshipClaim::DirectedAgent] {
            let v = human_authorship().evaluate(
                &contribution(&["src/main.rs"], TrustTier::Contributor),
                &[declares(claim.clone())],
            );
            assert_eq!(v.decision, Decision::Satisfied, "{claim:?}: {:?}", v.unmet);
            // Authorship counts toward the ranking like any other requirement.
            // Without this the decision is right and the confidence is wrong,
            // which is invisible until a queue sorts by it — and a queue sorted
            // by a number nobody checks is the failure ranking exists to avoid.
            assert!(
                (v.rank.confidence - 1.0).abs() < f64::EPSILON,
                "{claim:?}: confidence {}",
                v.rank.confidence
            );
        }
    }

    #[test]
    fn an_unmet_authorship_requirement_lowers_confidence_rather_than_being_free() {
        let v = human_authorship().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[declares(AuthorshipClaim::Generated)],
        );
        assert!(
            v.rank.confidence.abs() < f64::EPSILON,
            "confidence {}",
            v.rank.confidence
        );
    }

    #[test]
    fn generated_work_is_refused_when_it_is_declared_honestly() {
        // The flagship case: a contribution that says what it is, and a project
        // that has decided it does not want that.
        let v = human_authorship().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[declares(AuthorshipClaim::Generated)],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::WrongAuthorship);
    }

    #[test]
    fn one_honest_declaration_is_not_laundered_by_another() {
        // A contribution carrying both a human claim and a generated claim
        // contains generated work. Accepting it because *some* declaration is
        // acceptable would make the rule trivially satisfiable by attaching one
        // extra record.
        let v = human_authorship().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[
                declares(AuthorshipClaim::Human),
                declares(AuthorshipClaim::Generated),
            ],
        );
        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].reason, UnmetReason::WrongAuthorship);
    }

    #[test]
    fn a_machine_cannot_declare_authorship() {
        // Only an attested record carries a declaration. Honouring one on a
        // deterministic or assessed record would let a test runner or a model
        // assert human authorship, which is the thing this exists to prevent.
        for class in [EvidenceClass::Deterministic, EvidenceClass::Assessed] {
            let mut machine = declares(AuthorshipClaim::Human);
            machine.class = class;
            let v = human_authorship().evaluate(
                &contribution(&["src/main.rs"], TrustTier::Contributor),
                &[machine],
            );
            assert_eq!(v.decision, Decision::Blocked, "{class:?}");
            assert_eq!(v.unmet[0].reason, UnmetReason::Undeclared, "{class:?}");
        }
    }

    #[test]
    fn a_declaration_about_another_revision_does_not_count() {
        let mut stale = declares(AuthorshipClaim::Human);
        stale.subject_digest = "an-older-revision".into();
        let v = human_authorship().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[stale],
        );
        assert_eq!(v.unmet[0].reason, UnmetReason::Undeclared);
    }

    #[test]
    fn matching_rules_intersect_rather_than_union_what_they_accept() {
        // Weak-rule shopping, applied to authorship. A contribution matching a
        // permissive rule and a strict one is held to the strict one.
        let src = r#"
version: 0
rules:
  - name: broad
    when:
      paths: ["**"]
    require:
      authorship: [human, directed_agent, generated]
  - name: sensitive
    when:
      paths: ["src/**"]
    require:
      authorship: [human]
"#;
        let policy = || Policy::from_yaml(src).unwrap().compile().unwrap();
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);

        assert_eq!(
            policy()
                .evaluate(&c, &[declares(AuthorshipClaim::DirectedAgent)])
                .unmet[0]
                .reason,
            UnmetReason::WrongAuthorship,
            "the stricter rule decides"
        );
        assert_eq!(
            policy()
                .evaluate(&c, &[declares(AuthorshipClaim::Human)])
                .decision,
            Decision::Satisfied
        );
    }

    #[test]
    fn a_derived_claim_keeps_its_source_and_is_named_only_by_kind() {
        let src = r#"
version: 0
rules:
  - name: allow-derived
    when:
      paths: ["**"]
    require:
      authorship: [derived]
"#;
        let claim = AuthorshipClaim::Derived {
            source: "https://github.com/acme/widgets".into(),
            license: Some("Apache-2.0".into()),
        };
        let v = Policy::from_yaml(src).unwrap().compile().unwrap().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[declares(claim)],
        );
        assert_eq!(v.decision, Decision::Satisfied, "{:?}", v.unmet);
    }

    #[test]
    fn a_policy_saying_nothing_about_authorship_constrains_nothing() {
        // Existing policies must not change meaning under their feet.
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests"), approval("someone")],
        );
        assert_eq!(v.decision, Decision::Satisfied);
        assert!(v.unmet.is_empty());
    }

    // --- the governing policy is base and head together --------------------

    /// What a contribution ships when it deletes the rules that govern it.
    const GUTTED: &str = "version: 0\nrules: []\n";

    fn gutted() -> CompiledPolicy {
        Policy::from_yaml(GUTTED).unwrap().compile().unwrap()
    }

    fn governing() -> CompiledPolicy {
        Policy::from_yaml(POLICY)
            .unwrap()
            .labelled("governing")
            .compile()
            .unwrap()
    }

    #[test]
    fn a_contribution_cannot_escape_a_rule_by_deleting_it() {
        let c = contribution(&[".github/workflows/ci.yml"], TrustTier::Unknown);

        // The hole this exists to close, asserted rather than described: judged
        // by the document it ships, a contribution that demands nothing of
        // itself is satisfied, with no evidence and no standing.
        let escaped = gutted().evaluate(&c, &[]);
        assert_eq!(escaped.decision, Decision::Satisfied);
        assert!(escaped.matched_rules.is_empty());

        // Judged by the rules that were in force when it was proposed, it is not.
        let governed = CompiledPolicy::merged(vec![governing(), gutted()]).evaluate(&c, &[]);
        assert_eq!(governed.decision, Decision::Blocked);
        assert!(
            !governed.tier_satisfied,
            "the deleted tier bar still applies"
        );
        assert_eq!(
            governed.matched_rules,
            vec!["governing:baseline", "governing:ci-config"],
            "a verdict must name the document a rule came from, or the reader \
             cannot find the rule that blocked them"
        );
    }

    #[test]
    fn merging_is_never_weaker_than_either_input() {
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);
        let ev = [pass("build"), pass("tests")];

        let alone = Policy::from_yaml(POLICY).unwrap().compile().unwrap();
        assert_eq!(gutted().evaluate(&c, &ev).decision, Decision::Satisfied);
        assert_eq!(alone.evaluate(&c, &ev).decision, Decision::NeedsHuman);

        let merged = CompiledPolicy::merged(vec![
            Policy::from_yaml(POLICY).unwrap().compile().unwrap(),
            gutted(),
        ]);
        assert_eq!(
            merged.evaluate(&c, &ev).decision,
            Decision::NeedsHuman,
            "the stricter input decides"
        );
    }

    #[test]
    fn a_rule_a_contribution_adds_binds_that_contribution() {
        // The other half of the asymmetry. Tightening applies at once, so a rule
        // cannot be introduced and dodged in the same change; loosening applies
        // only once the change that loosens it has itself been accepted.
        let added = r#"
version: 0
rules:
  - name: new
    when:
      paths: ["**"]
    require:
      deterministic: [audit]
"#;
        let v = CompiledPolicy::merged(vec![
            gutted(),
            Policy::from_yaml(added).unwrap().compile().unwrap(),
        ])
        .evaluate(&contribution(&["src/main.rs"], TrustTier::Contributor), &[]);

        assert_eq!(v.decision, Decision::Blocked);
        assert_eq!(v.unmet[0].requirement, "audit");
    }

    #[test]
    fn a_policy_merged_with_itself_decides_the_same_way() {
        // Base and head are identical for every contribution that does not touch
        // the policy, which is nearly all of them. The common case must not
        // change meaning under anyone's feet.
        let c = contribution(&["src/main.rs"], TrustTier::Contributor);
        let ev = [pass("build"), approval("someone")];

        let once = policy().evaluate(&c, &ev);
        let twice = CompiledPolicy::merged(vec![policy(), policy()]).evaluate(&c, &ev);

        assert_eq!(once.decision, twice.decision);
        assert_eq!(once.unmet, twice.unmet);
        assert_eq!(once.approvals_present, twice.approvals_present);
        assert_eq!(once.tier_required, twice.tier_required);
        assert!((once.rank.confidence - twice.rank.confidence).abs() < f64::EPSILON);
    }

    #[test]
    fn merging_does_not_disturb_determinism() {
        let c = contribution(&[".github/workflows/ci.yml"], TrustTier::Maintainer);
        let ev = [
            pass("build"),
            pass("tests"),
            pass("workflow-audit"),
            approval("m"),
        ];
        let mut reversed = ev.clone();
        reversed.reverse();
        let build = || CompiledPolicy::merged(vec![governing(), policy()]);

        assert_eq!(
            serde_json::to_string(&build().evaluate(&c, &ev)).unwrap(),
            serde_json::to_string(&build().evaluate(&c, &reversed)).unwrap()
        );
    }

    #[test]
    fn without_the_constraint_approvals_behave_as_before() {
        // Existing policies must not change meaning under their feet.
        let v = policy().evaluate(
            &contribution(&["src/main.rs"], TrustTier::Contributor),
            &[pass("build"), pass("tests"), approval("someone")],
        );
        assert_eq!(v.approvals_present, 1);
        assert_eq!(v.decision, Decision::Satisfied);
    }
}
