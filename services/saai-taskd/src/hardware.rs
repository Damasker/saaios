//! One hardware campaign for an explicit `investigate_hardware` intent.
//! Free text and any other action id do not open a probe journal.

use std::path::Path;

use hdi_campaign::{
    campaign_from_action, confirmation_required, propose_display_adaptation, verify, RunReport,
    INVESTIGATE_HARDWARE,
};

#[derive(Debug)]
pub struct HardwareRun {
    pub summary: String,
    pub loaded_module: bool,
    pub discovery_gain: usize,
    pub identification_accuracy: f64,
    pub evidence_coverage: f64,
    pub probe_efficiency: f64,
    pub verdict: Option<String>,
    pub asks_user: bool,
}

#[derive(Debug)]
pub enum HardwareStart {
    NotThisAction,
    MissingDirectory,
    Finished(HardwareRun),
    Failed,
}

pub fn start_hardware_campaign(
    action_id: &str,
    redacted: Option<&Path>,
    campaign: &Path,
    key: Option<&Path>,
) -> HardwareStart {
    if action_id != INVESTIGATE_HARDWARE {
        return HardwareStart::NotThisAction;
    }
    let Some(redacted) = redacted else {
        return HardwareStart::MissingDirectory;
    };
    if !redacted.is_dir() {
        return HardwareStart::MissingDirectory;
    }
    let report = match campaign_from_action(action_id, redacted, campaign) {
        Ok(Some(report)) => report,
        Ok(None) => return HardwareStart::NotThisAction,
        Err(error) => {
            eprintln!("saai-taskd: hardware campaign failed: {error}");
            return HardwareStart::Failed;
        }
    };
    let (metrics, verdict) = score(&report, redacted, key);
    let proposal = propose_display_adaptation(&report.claims);
    let loaded_module = proposal.loaded_module;
    let asks_user = confirmation_required(&proposal);
    let verdict_text = verdict.as_deref().unwrap_or("unread").to_string();
    HardwareStart::Finished(HardwareRun {
        summary: format!(
            "HDI gain={} efficiency={} accuracy={} coverage={} verdict={verdict_text} loaded_module={loaded_module}",
            metrics.discovery_gain,
            metrics.probe_efficiency,
            metrics.identification_accuracy,
            metrics.evidence_coverage,
        ),
        loaded_module,
        discovery_gain: metrics.discovery_gain,
        identification_accuracy: metrics.identification_accuracy,
        evidence_coverage: metrics.evidence_coverage,
        probe_efficiency: metrics.probe_efficiency,
        verdict,
        asks_user,
    })
}

fn score(
    report: &RunReport,
    redacted: &Path,
    key: Option<&Path>,
) -> (hdi_campaign::Metrics, Option<String>) {
    let Some(key) = key else {
        return (report.metrics.clone(), None);
    };
    if key.starts_with(redacted) {
        return (report.metrics.clone(), None);
    }
    match verify(redacted, key) {
        Ok(verification) => {
            let metrics = report.metrics.with_stand_key(
                &report.claims,
                &verification.confirmed_for(&verification.machine_id),
            );
            (metrics, Some(format!("{:?}", verification.verdict)))
        }
        Err(error) => {
            eprintln!("saai-taskd: hardware verify failed: {error}");
            (report.metrics.clone(), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{start_hardware_campaign, HardwareStart};
    use hdi_campaign::INVESTIGATE_HARDWARE;
    use std::path::Path;

    fn exec_fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/hdi-exec/tests/fixtures/exec-root")
    }

    #[test]
    fn explicit_fixture_finishes_one_campaign() {
        let campaign = tempfile::tempdir().expect("campaign");
        let started = start_hardware_campaign(
            INVESTIGATE_HARDWARE,
            Some(&exec_fixture()),
            campaign.path(),
            None,
        );
        let HardwareStart::Finished(run) = started else {
            panic!("expected a finished campaign");
        };
        assert!(run.summary.contains("gain="));
        assert!(run.summary.contains("loaded_module=false"));
        assert!(!run.loaded_module);
        assert!(run.asks_user);
        assert!(campaign.path().join("journal.jsonl").is_file());
        let journal = std::fs::read_to_string(campaign.path().join("journal.jsonl")).unwrap();
        assert!(!journal.contains("insmod"));
        assert!(!journal.contains("modprobe"));
    }

    #[test]
    fn bound_driver_finishes_without_a_question() {
        let campaign = tempfile::tempdir().expect("campaign");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/hdi-campaign/tests/fixtures/drm-bound");
        let started = start_hardware_campaign(
            INVESTIGATE_HARDWARE,
            Some(&fixture),
            campaign.path(),
            None,
        );
        let HardwareStart::Finished(run) = started else {
            panic!("expected a finished campaign");
        };
        assert!(!run.asks_user);
        assert!(!run.loaded_module);
    }

    #[test]
    fn free_text_does_not_open_a_journal() {
        let campaign = tempfile::tempdir().expect("campaign");
        let started = start_hardware_campaign(
            "look at the gpu",
            Some(&exec_fixture()),
            campaign.path(),
            None,
        );
        assert!(matches!(started, HardwareStart::NotThisAction));
        assert!(!campaign.path().join("journal.jsonl").exists());
    }

    #[test]
    fn missing_directory_does_not_open_a_journal() {
        let campaign = tempfile::tempdir().expect("campaign");
        let started = start_hardware_campaign(INVESTIGATE_HARDWARE, None, campaign.path(), None);
        assert!(matches!(started, HardwareStart::MissingDirectory));
        assert!(!campaign.path().join("journal.jsonl").exists());
    }
}
