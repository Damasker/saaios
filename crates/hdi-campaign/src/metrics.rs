use hdi_contract::{discovery_gain, Claim};
use serde::Serialize;

/// One machine cannot demonstrate transfer. HDI-04 owns the two-machine check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Transferability {
    NotTransferred,
}

/// ADR-426 metrics. Every numeric field is finite.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    pub identification_accuracy: f64,
    pub evidence_coverage: f64,
    pub discovery_gain: usize,
    pub false_hypotheses: usize,
    pub probe_efficiency: f64,
    pub transferability: Transferability,
}

impl Metrics {
    pub fn from_run(
        investigator: &[Claim],
        control: &[Claim],
        executed: u32,
        key: &[Claim],
    ) -> Self {
        let discovery_gain = discovery_gain(investigator, control);
        Self {
            identification_accuracy: identification_accuracy(investigator, key),
            evidence_coverage: evidence_coverage(investigator),
            discovery_gain,
            false_hypotheses: 0,
            probe_efficiency: probe_efficiency(discovery_gain, executed),
            transferability: Transferability::NotTransferred,
        }
    }

    pub fn is_finite(&self) -> bool {
        self.identification_accuracy.is_finite()
            && self.evidence_coverage.is_finite()
            && self.probe_efficiency.is_finite()
    }
}

pub fn identification_accuracy(claims: &[Claim], key: &[Claim]) -> f64 {
    if key.is_empty() {
        return 0.0;
    }
    let matched = key
        .iter()
        .filter(|expected| claims.iter().any(|claim| claim.same_fact(expected)))
        .count();
    matched as f64 / key.len() as f64
}

pub fn evidence_coverage(claims: &[Claim]) -> f64 {
    if claims.is_empty() {
        return 0.0;
    }
    let covered = claims
        .iter()
        .filter(|claim| {
            !claim.evidence_ids.is_empty() && claim.evidence_ids.iter().all(|id| !id.is_empty())
        })
        .count();
    covered as f64 / claims.len() as f64
}

pub fn probe_efficiency(gain: usize, executed: u32) -> f64 {
    if executed == 0 {
        0.0
    } else {
        gain as f64 / f64::from(executed)
    }
}
