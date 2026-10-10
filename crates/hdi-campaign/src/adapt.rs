//! Display adaptation proposal. It never loads a module.

use hdi_contract::{Claim, ClaimKind};

const CARD_SUBJECT: &str = "drm:card0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayAdaptation {
    AlreadyBound { driver: String },
    DriverGap,
    Insufficient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptationProposal {
    pub display: DisplayAdaptation,
    pub loaded_module: bool,
    pub gap: Option<Claim>,
}

/// A bound DRM driver ends the proposal. A present card with no driver is a
/// gap record. Nothing here is an executable load.
pub fn propose_display_adaptation(claims: &[Claim]) -> AdaptationProposal {
    if let Some(driver) = bound_driver(claims) {
        return AdaptationProposal {
            display: DisplayAdaptation::AlreadyBound { driver },
            loaded_module: false,
            gap: None,
        };
    }
    if let Some(evidence) = card_evidence(claims) {
        return AdaptationProposal {
            display: DisplayAdaptation::DriverGap,
            loaded_module: false,
            gap: Some(Claim::new(
                ClaimKind::DriverGap,
                CARD_SUBJECT,
                "unbound",
                evidence,
            )),
        };
    }
    AdaptationProposal {
        display: DisplayAdaptation::Insufficient,
        loaded_module: false,
        gap: None,
    }
}

fn bound_driver(claims: &[Claim]) -> Option<String> {
    claims.iter().find_map(|claim| {
        if claim.kind != ClaimKind::DriverBound || claim.subject != CARD_SUBJECT {
            return None;
        }
        if executable(&claim.value) || claim.value.is_empty() {
            return None;
        }
        Some(claim.value.clone())
    })
}

fn card_evidence(claims: &[Claim]) -> Option<Vec<String>> {
    let claim = claims.iter().find(|claim| {
        claim.kind == ClaimKind::DevicePresent
            && claim.subject == CARD_SUBJECT
            && claim.value == "card0"
            && !claim.evidence_ids.is_empty()
            && claim.evidence_ids.iter().all(|id| !id.is_empty())
    })?;
    Some(claim.evidence_ids.clone())
}

fn executable(text: &str) -> bool {
    text.contains("insmod") || text.contains("modprobe") || text.contains(".ko")
}
