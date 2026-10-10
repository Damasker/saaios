use serde::Deserialize;

use crate::error::ContractError;
use crate::probe::ProbeRequest;

/// Semantic action that opens one hardware investigation.
pub const INVESTIGATE_HARDWARE: &str = "investigate_hardware";

/// Maximum investigator passes in one campaign.
pub const MAX_PASSES: u32 = 5;

/// Maximum probes an executor may be asked to run.
pub const MAX_EXECUTED_PROBES: u32 = 5;

/// Intent fields the contract reads. Extra fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentDocument {
    pub semantic_action_id: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub precondition_id: Option<String>,
}

/// One investigation. Constructed only from an intent document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campaign {
    intent: IntentDocument,
    executed: u32,
}

impl Campaign {
    /// `document` is the raw intent JSON. Empty input does not open a campaign.
    pub fn open(document: &str) -> Result<Self, ContractError> {
        if document.trim().is_empty() {
            return Err(ContractError::MissingIntent);
        }
        let intent: IntentDocument =
            serde_json::from_str(document).map_err(|_| ContractError::InvalidIntent)?;
        let action = intent
            .semantic_action_id
            .as_deref()
            .filter(|action| !action.is_empty())
            .ok_or(ContractError::MissingAction)?;
        if action != INVESTIGATE_HARDWARE {
            return Err(ContractError::OtherAction {
                action: action.to_string(),
            });
        }
        match intent.source.as_deref() {
            Some("orb") => {}
            Some("worker") => {
                let precondition = intent.precondition_id.as_deref().unwrap_or("");
                if precondition.is_empty() {
                    return Err(ContractError::WorkerRequiresPrecondition);
                }
            }
            _ => return Err(ContractError::InvalidSource),
        }
        Ok(Self {
            intent,
            executed: 0,
        })
    }

    pub fn intent(&self) -> &IntentDocument {
        &self.intent
    }

    pub fn executed(&self) -> u32 {
        self.executed
    }

    /// Validate `raw` and reserve one execution slot.
    ///
    /// A pass above [`MAX_PASSES`] or a slot past [`MAX_EXECUTED_PROBES`]
    /// returns [`ContractError::PassLimit`] without incrementing the counter.
    /// The caller must not invoke an executor on that error.
    pub fn accept(&mut self, raw: &str) -> Result<ProbeRequest, ContractError> {
        let request = ProbeRequest::parse(raw)?;
        if request.pass == 0 || request.pass > MAX_PASSES || self.executed >= MAX_EXECUTED_PROBES {
            return Err(ContractError::PassLimit);
        }
        self.executed += 1;
        Ok(request)
    }
}
