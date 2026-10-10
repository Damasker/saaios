use serde::Deserialize;

use crate::error::ContractError;

/// Comparable fact. Free text is not part of equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    DevicePresent,
    DeviceAbsent,
    DriverBound,
    DriverUnbound,
    Causal,
    InsufficientEvidence,
    NegativeRecipe,
    DriverGap,
}

/// One statement. `hypothesis` is kept for audit and ignored by [`Claim::same_fact`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub kind: ClaimKind,
    pub subject: String,
    pub value: String,
    pub evidence_ids: Vec<String>,
    pub hypothesis: Option<String>,
}

impl Claim {
    pub fn new(
        kind: ClaimKind,
        subject: impl Into<String>,
        value: impl Into<String>,
        evidence_ids: Vec<String>,
    ) -> Self {
        Self {
            kind,
            subject: subject.into(),
            value: value.into(),
            evidence_ids,
            hypothesis: None,
        }
    }

    pub fn with_hypothesis(mut self, hypothesis: impl Into<String>) -> Self {
        self.hypothesis = Some(hypothesis.into());
        self
    }

    /// ADR-426 equality: kind, subject, and value. Hypothesis and evidence ids do not count.
    pub fn same_fact(&self, other: &Claim) -> bool {
        self.kind == other.kind && self.subject == other.subject && self.value == other.value
    }
}

/// Confirmed statements. A claim without evidence ids is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Knowledge {
    records: Vec<Claim>,
}

impl Knowledge {
    pub fn try_record(&mut self, claim: Claim) -> Result<(), ContractError> {
        if claim.evidence_ids.is_empty() || claim.evidence_ids.iter().any(String::is_empty) {
            return Err(ContractError::NoEvidence);
        }
        self.records.push(claim);
        Ok(())
    }

    pub fn records(&self) -> &[Claim] {
        &self.records
    }
}

/// Confirmed investigator claims whose fact is not already in `control`.
pub fn discovery_gain(investigator: &[Claim], control: &[Claim]) -> usize {
    investigator
        .iter()
        .filter(|claim| !claim.evidence_ids.is_empty())
        .filter(|claim| !control.iter().any(|known| claim.same_fact(known)))
        .count()
}
