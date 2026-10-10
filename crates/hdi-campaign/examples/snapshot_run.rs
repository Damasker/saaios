//! One conscious campaign over an already redacted snapshot directory.
//!
//! The directory must contain `intent.txt` with `investigate_hardware`
//! and a `redacted/` bundle. The optional key path is read only by the
//! verifier after the loop. It is not passed to the investigator.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use hdi_campaign::{propose_display_adaptation, run_campaign, verify, Hunter, RunStatus};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(root) = args.next() else {
        eprintln!("usage: snapshot_run <snapshot-dir> [verifier-key]");
        return ExitCode::from(2);
    };
    let key = args.next().map(PathBuf::from);
    let root = PathBuf::from(root);
    let intent_path = root.join("intent.txt");
    let intent = match fs::read_to_string(&intent_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("no intent file: {error}");
            return ExitCode::from(2);
        }
    };
    if intent.trim_start_matches('\u{feff}').trim() != "investigate_hardware" {
        eprintln!("campaign refused: intent file is not investigate_hardware");
        return ExitCode::from(2);
    }

    let fixture = root.join("redacted");
    let campaign = root.join("campaign");
    let mut hunter = Hunter::new();
    let report = match run_campaign(&fixture, &campaign, &["pci.txt"], &mut hunter) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("campaign failed: {error}");
            return ExitCode::from(1);
        }
    };
    if report.status != RunStatus::Completed {
        eprintln!("campaign did not complete");
        return ExitCode::from(1);
    }
    let mut metrics = report.metrics.clone();
    let verdict = match key.as_deref() {
        None => None,
        Some(key) if key.starts_with(&fixture) => {
            eprintln!("verifier key must stay outside redacted/");
            return ExitCode::from(2);
        }
        Some(key) => match verify(&fixture, key) {
            Ok(verification) => {
                metrics = metrics.with_stand_key(
                    &report.claims,
                    &verification.confirmed_for(&verification.machine_id),
                );
                Some(verification.verdict)
            }
            Err(error) => {
                eprintln!("verify failed: {error}");
                return ExitCode::from(1);
            }
        },
    };
    println!(
        "executed={} calls={} gain={} pass_gain={:?} efficiency={} accuracy={} coverage={} transferability={:?}",
        report.executed,
        report.investigator_calls,
        metrics.discovery_gain,
        report.pass_gain,
        metrics.probe_efficiency,
        metrics.identification_accuracy,
        metrics.evidence_coverage,
        metrics.transferability,
    );
    println!("control_facts={}", report.control_facts);
    println!("finite={}", metrics.is_finite());
    let proposal = propose_display_adaptation(&report.claims);
    println!(
        "adaptation={:?} loaded_module={} gap={}",
        proposal.display,
        proposal.loaded_module,
        proposal.gap.is_some()
    );

    if let Some(verdict) = verdict {
        println!("verdict={verdict:?}");
    }
    ExitCode::SUCCESS
}
