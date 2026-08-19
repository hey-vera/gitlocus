// SPDX-License-Identifier: Apache-2.0
//! `locus` — evaluate a contribution against a repository's own policy.
//!
//! The same evaluation runs here and in CI, out of the same `gitlocus-core` crate,
//! so a contributor can find out locally exactly what the gate will say. If these
//! two ever disagree, that is a bug in this project and not a quirk of CI.

mod git;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use gitlocus_core::{
    Actor, ActorKind, Contribution, Evidence, EvidenceClass, Outcome, Policy, TrustTier, VouchList,
    VouchStatus,
};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Command line interface.
#[derive(Parser)]
#[command(
    name = "locus",
    version,
    about = "Evaluate contributions against a repository's own policy.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Top-level subcommands.
#[derive(Subcommand)]
enum Command {
    /// Evaluate a contribution and report what it still needs.
    Verify {
        /// Path to the policy document.
        #[arg(long, default_value = ".gitlocus/policy.yml")]
        policy: PathBuf,
        /// Path to the contribution JSON, or `-` for stdin.
        #[arg(long)]
        contribution: PathBuf,
        /// Path to a JSON array of evidence. Defaults to an empty set.
        #[arg(long)]
        evidence: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Parse and compile a policy without evaluating anything.
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// Produce a single piece of evidence on stdout.
    Evidence {
        #[command(subcommand)]
        action: EvidenceAction,
    },
    /// Describe the change between two revisions, reading the facts from git.
    ///
    /// This is what makes the tool usable outside the repository that wrote it:
    /// without it, adopting GitLocus means hand-writing the contribution
    /// document, and nobody adopts a format they have to author by hand.
    Contribution {
        /// Revision the change is proposed against.
        #[arg(long)]
        base: String,
        /// The proposed revision.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Canonical repository identifier. Derived from the remote when omitted.
        #[arg(long)]
        repository: Option<String>,
        /// Remote to derive the repository identifier from.
        #[arg(long, default_value = "origin")]
        remote: String,
        /// Identity of the actor. Defaults to the head commit's author email.
        #[arg(long)]
        actor: Option<String>,
        /// Trust tier to record.
        #[arg(long, value_enum, default_value_t = TierArg::Unknown)]
        tier: TierArg,
        /// Agent implementation identifier. Makes this agent-produced work.
        #[arg(long)]
        agent: Option<String>,
        /// Human answerable for the agent's work. With --agent, makes a pair.
        #[arg(long)]
        operator: Option<String>,
        /// OIDC subject or key the actor's signatures verify against.
        #[arg(long)]
        key_binding: Option<String>,
        /// Trust file to consult, in the format used by mitchellh/vouch.
        #[arg(long)]
        vouched_file: Option<PathBuf>,
        /// Platform to match identities against in the trust file.
        #[arg(long, default_value = "github")]
        platform: String,
    },
    /// Ask what a trust file says about an identity.
    Vouch {
        #[command(subcommand)]
        action: VouchAction,
    },
}

/// Policy subcommands.
#[derive(Subcommand)]
enum PolicyAction {
    /// Check that a policy document is valid.
    Check {
        /// Path to the policy document.
        #[arg(long, default_value = ".gitlocus/policy.yml")]
        policy: PathBuf,
    },
}

/// Evidence subcommands.
#[derive(Subcommand)]
enum EvidenceAction {
    /// Emit one evidence record as JSON.
    Emit {
        /// Requirement name this evidence answers to.
        #[arg(long)]
        kind: String,
        /// How much the claim is worth.
        #[arg(long, value_enum)]
        class: ClassArg,
        /// What it reports.
        #[arg(long, value_enum)]
        outcome: OutcomeArg,
        /// Revision the claim is about.
        #[arg(long)]
        subject: String,
        /// Identifier of whatever produced the claim.
        #[arg(long)]
        produced_by: String,
        /// RFC 3339 timestamp.
        #[arg(long)]
        produced_at: String,
        /// Where the underlying run can be inspected.
        #[arg(long)]
        source_uri: Option<String>,
        /// Free-form detail.
        #[arg(long)]
        summary: Option<String>,
    },
}

/// Vouch subcommands.
#[derive(Subcommand)]
enum VouchAction {
    /// Report what a trust file says about an identity.
    Check {
        /// Path to the trust file.
        #[arg(long, default_value = "VOUCHED.td")]
        file: PathBuf,
        /// Identity to look up.
        #[arg(long)]
        user: String,
        /// Platform the identity belongs to.
        #[arg(long, default_value = "github")]
        platform: String,
    },
}

/// Trust tier, as a CLI argument.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum TierArg {
    /// No established standing.
    Unknown,
    /// Someone the repository trusts will speak for them.
    Vouched,
    /// Has landed changes here before.
    Contributor,
    /// Carries review authority here.
    Maintainer,
}

impl From<TierArg> for TrustTier {
    fn from(value: TierArg) -> Self {
        match value {
            TierArg::Unknown => Self::Unknown,
            TierArg::Vouched => Self::Vouched,
            TierArg::Contributor => Self::Contributor,
            TierArg::Maintainer => Self::Maintainer,
        }
    }
}

/// Output format for `verify`.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable.
    Text,
    /// Machine-readable verdict.
    Json,
}

/// Evidence class, as a CLI argument.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ClassArg {
    /// Reproducible by a third party.
    Deterministic,
    /// A heuristic or model judgement.
    Assessed,
    /// A human took responsibility.
    Attested,
}

impl From<ClassArg> for EvidenceClass {
    fn from(value: ClassArg) -> Self {
        match value {
            ClassArg::Deterministic => Self::Deterministic,
            ClassArg::Assessed => Self::Assessed,
            ClassArg::Attested => Self::Attested,
        }
    }
}

/// Outcome, as a CLI argument.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutcomeArg {
    /// Succeeded.
    Pass,
    /// Failed.
    Fail,
    /// No answer reached.
    Inconclusive,
}

impl From<OutcomeArg> for Outcome {
    fn from(value: OutcomeArg) -> Self {
        match value {
            OutcomeArg::Pass => Self::Pass,
            OutcomeArg::Fail => Self::Fail,
            OutcomeArg::Inconclusive => Self::Inconclusive,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("locus: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Verify {
            policy,
            contribution,
            evidence,
            format,
        } => verify(&policy, &contribution, evidence.as_deref(), format),
        Command::Policy { action } => {
            let PolicyAction::Check { policy } = action;
            check_policy(&policy)
        }
        Command::Evidence { action } => emit_evidence(action),
        Command::Contribution {
            base,
            head,
            repository,
            remote,
            actor,
            tier,
            agent,
            operator,
            key_binding,
            vouched_file,
            platform,
        } => describe_contribution(ContributionArgs {
            base,
            head,
            repository,
            remote,
            actor,
            tier,
            agent,
            operator,
            key_binding,
            vouched_file,
            platform,
        }),
        Command::Vouch { action } => check_vouch(action),
    }
}

/// Inputs to `locus contribution`, grouped so the function signature stays legible.
struct ContributionArgs {
    base: String,
    head: String,
    repository: Option<String>,
    remote: String,
    actor: Option<String>,
    tier: TierArg,
    agent: Option<String>,
    operator: Option<String>,
    key_binding: Option<String>,
    vouched_file: Option<PathBuf>,
    platform: String,
}

fn describe_contribution(args: ContributionArgs) -> Result<ExitCode> {
    let base_digest = git::resolve(&args.base)?;
    let head_digest = git::resolve(&args.head)?;
    let changed_paths = git::changed_paths(&base_digest, &head_digest)?;

    let repository = match args.repository {
        Some(explicit) => explicit,
        None => git::repository_from_remote(&args.remote)?,
    };

    let id = match args.actor {
        Some(explicit) => explicit,
        None => git::author_email(&head_digest)?,
    };

    let kind = match (args.agent, args.operator) {
        (Some(implementation), Some(operator)) => ActorKind::Pair {
            implementation,
            operator,
        },
        (Some(implementation), None) => ActorKind::Agent { implementation },
        // An operator without an agent is a human doing their own work.
        (None, _) => ActorKind::Human,
    };

    let mut tier: TrustTier = args.tier.into();

    if let Some(path) = args.vouched_file {
        let list = VouchList::parse(&read(&path)?);
        match list.status(&args.platform, &id) {
            VouchStatus::Denounced => {
                // A denouncement caps the tier no matter what was asked for. A
                // trust file and a command-line flag disagreeing about someone
                // is a contradiction, and the safe reading of a contradiction
                // about trust is the restrictive one. It is loud rather than
                // silent because a downgrade nobody notices is a bug.
                let reason = list
                    .reason(&args.platform, &id)
                    .unwrap_or("no reason given");
                eprintln!(
                    "locus: {id} is denounced in {} ({reason}); recording tier unknown",
                    path.display()
                );
                tier = TrustTier::Unknown;
            }
            VouchStatus::Vouched if tier == TrustTier::Unknown => {
                tier = TrustTier::Vouched;
            }
            // An actor who already holds a higher tier is not demoted by merely
            // appearing in the file, and an absent actor is left as asked.
            VouchStatus::Vouched | VouchStatus::Unknown => {}
        }
    }

    let contribution = Contribution {
        repository,
        base_digest,
        head_digest,
        actor: Actor {
            id,
            kind,
            tier,
            key_binding: args.key_binding,
        },
        changed_paths,
        forge_ref: None,
    };

    println!("{}", serde_json::to_string_pretty(&contribution)?);
    Ok(ExitCode::SUCCESS)
}

fn check_vouch(action: VouchAction) -> Result<ExitCode> {
    let VouchAction::Check {
        file,
        user,
        platform,
    } = action;

    let list = VouchList::parse(&read(&file)?);
    let status = list.status(&platform, &user);
    let tier = gitlocus_core::vouch::tier_for(status);

    let label = match status {
        VouchStatus::Vouched => "vouched",
        VouchStatus::Denounced => "denounced",
        VouchStatus::Unknown => "not mentioned",
    };
    print!("{platform}:{user} is {label}");
    if let Some(reason) = list.reason(&platform, &user) {
        print!(" ({reason})");
    }
    println!(" — tier {tier:?}");

    // Non-zero for denounced, so a shell can branch on it without parsing text.
    Ok(if status == VouchStatus::Denounced {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn verify(
    policy_path: &Path,
    contribution_path: &Path,
    evidence_path: Option<&Path>,
    format: Format,
) -> Result<ExitCode> {
    let policy = load_policy(policy_path)?;

    let contribution: Contribution = serde_json::from_str(&read(contribution_path)?)
        .with_context(|| format!("parsing contribution from {}", contribution_path.display()))?;

    let evidence: Vec<Evidence> = match evidence_path {
        Some(path) => serde_json::from_str(&read(path)?)
            .with_context(|| format!("parsing evidence from {}", path.display()))?,
        None => Vec::new(),
    };

    let verdict = policy.evaluate(&contribution, &evidence);

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&verdict)?),
        Format::Text => {
            println!("contribution : {}", verdict.contribution);
            println!("revision     : {}", verdict.subject_digest);
            println!("rules matched: {}", join(&verdict.matched_rules));
            println!("verdict      : {}", verdict.headline());
            for unmet in &verdict.unmet {
                println!("  unmet      : {} ({:?})", unmet.requirement, unmet.reason);
            }
            for note in &verdict.advisory {
                println!("  advisory   : {note} [not binding]");
            }
            println!(
                "rank         : confidence {:.2}, {} approval(s) outstanding",
                verdict.rank.confidence, verdict.rank.human_cost
            );
        }
    }

    Ok(if verdict.is_mergeable() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn check_policy(path: &Path) -> Result<ExitCode> {
    load_policy(path)?;
    println!("{}: ok", path.display());
    Ok(ExitCode::SUCCESS)
}

fn emit_evidence(action: EvidenceAction) -> Result<ExitCode> {
    let EvidenceAction::Emit {
        kind,
        class,
        outcome,
        subject,
        produced_by,
        produced_at,
        source_uri,
        summary,
    } = action;

    let evidence = Evidence {
        kind,
        class: class.into(),
        outcome: outcome.into(),
        subject_digest: subject,
        produced_by,
        produced_at,
        source_uri,
        summary,
        // Emitting a record does not sign it. A signer only ever comes from
        // verifying a bundle; see the note on the field itself.
        signer: None,
    };

    println!("{}", serde_json::to_string(&evidence)?);
    Ok(ExitCode::SUCCESS)
}

fn load_policy(path: &Path) -> Result<gitlocus_core::policy::CompiledPolicy> {
    let src = read(path)?;
    Policy::from_yaml(&src)
        .with_context(|| format!("parsing policy at {}", path.display()))?
        .compile()
        .with_context(|| format!("compiling policy at {}", path.display()))
}

/// Read a file, or stdin when the path is `-`.
fn read(path: &Path) -> Result<String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("reading stdin")?;
        return Ok(buf);
    }
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn join(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
