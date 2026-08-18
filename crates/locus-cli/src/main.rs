// SPDX-License-Identifier: Apache-2.0
//! `locus` — evaluate a contribution against a repository's own policy.
//!
//! The same evaluation runs here and in CI, out of the same `locus-core` crate,
//! so a contributor can find out locally exactly what the gate will say. If these
//! two ever disagree, that is a bug in this project and not a quirk of CI.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use locus_core::{Contribution, Evidence, EvidenceClass, Outcome, Policy};
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
    }
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
    };

    println!("{}", serde_json::to_string(&evidence)?);
    Ok(ExitCode::SUCCESS)
}

fn load_policy(path: &Path) -> Result<locus_core::policy::CompiledPolicy> {
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
