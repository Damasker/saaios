//! Conscious entry. A campaign starts only from an explicit
//! `investigate_hardware` intent. The shipped graphics precondition is off.

use std::collections::HashSet;

use intent_resolution::{
    resolve_deterministic, ActionResolution, AllowedContext, IntentInput, ResolutionOutcome,
    ResolveAttempt,
};

pub const INVESTIGATE_HARDWARE: &str = "investigate_hardware";
pub const ADAPT_HARDWARE: &str = "adapt_hardware";
pub const GRAPHICS_PRECONDITION: &str = "graphics_class_incomplete";

/// Shipped rules. `enabled` stays false until a later sprint turns one on.
pub const SHIPPED_PRECONDITIONS: &[Precondition] = &[Precondition {
    id: GRAPHICS_PRECONDITION,
    enabled: false,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Precondition {
    pub id: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentAuthor {
    Orb,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareIntent {
    pub semantic_action_id: String,
    pub author: IntentAuthor,
    pub precondition_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignTicket {
    pub intent_index: usize,
}

/// Same words as ADR-260. This gate does not execute tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareVerdict {
    Allow,
    AskUser,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adaptation {
    pub loaded_module: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDecision {
    pub started_campaign: bool,
    pub resolved_action: Option<String>,
}

#[derive(Debug, Default)]
pub struct IntentGate {
    preconditions: Vec<Precondition>,
    intents: Vec<HardwareIntent>,
    campaigns: Vec<CampaignTicket>,
    seen_observations: HashSet<String>,
}

impl IntentGate {
    pub fn new() -> Self {
        Self {
            preconditions: SHIPPED_PRECONDITIONS.to_vec(),
            ..Self::default()
        }
    }

    pub fn intents(&self) -> &[HardwareIntent] {
        &self.intents
    }

    pub fn campaigns(&self) -> &[CampaignTicket] {
        &self.campaigns
    }

    /// Test-only switch. Shipped data is unchanged.
    pub fn enable_for_test(&mut self, id: &str) {
        if let Some(rule) = self.preconditions.iter_mut().find(|rule| rule.id == id) {
            rule.enabled = true;
        }
    }

    pub fn submit(&mut self, input: &IntentInput, allowed: &AllowedContext) -> GateDecision {
        let attempt = resolve_deterministic(input, allowed);
        let action = action_of(&attempt);
        let started = action.as_deref() == Some(INVESTIGATE_HARDWARE);
        if started {
            let index = self.intents.len();
            self.intents.push(HardwareIntent {
                semantic_action_id: INVESTIGATE_HARDWARE.to_string(),
                author: IntentAuthor::Orb,
                precondition_id: None,
            });
            self.campaigns.push(CampaignTicket {
                intent_index: index,
            });
        }
        GateDecision {
            started_campaign: started,
            resolved_action: action,
        }
    }

    /// Boot, udev, and disabled rules write nothing.
    /// An enabled rule writes one worker intent per distinct observation.
    pub fn observe(&mut self, kind: &str, fingerprint: &str) -> usize {
        if kind == "boot" {
            return 0;
        }
        let Some(rule) = self.preconditions.iter().find(|rule| rule.id == kind) else {
            return 0;
        };
        if !rule.enabled {
            return 0;
        }
        let key = format!("{kind}\0{fingerprint}");
        if !self.seen_observations.insert(key) {
            return 0;
        }
        let index = self.intents.len();
        self.intents.push(HardwareIntent {
            semantic_action_id: INVESTIGATE_HARDWARE.to_string(),
            author: IntentAuthor::Worker,
            precondition_id: Some(kind.to_string()),
        });
        self.campaigns.push(CampaignTicket {
            intent_index: index,
        });
        1
    }

    /// AskUser stops here. No confirmation is accepted and no module is loaded.
    pub fn request_adaptation(&self, action_id: &str) -> (HardwareVerdict, Adaptation) {
        let verdict = match action_id {
            ADAPT_HARDWARE => HardwareVerdict::AskUser,
            INVESTIGATE_HARDWARE => HardwareVerdict::Allow,
            _ => HardwareVerdict::Deny,
        };
        (
            verdict,
            Adaptation {
                loaded_module: false,
            },
        )
    }
}

fn action_of(attempt: &ResolveAttempt) -> Option<String> {
    match attempt {
        ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Action(ActionResolution { action_id, .. }),
            ..
        } => Some(action_id.clone()),
        _ => None,
    }
}
