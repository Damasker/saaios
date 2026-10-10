//! HDI campaign contract (ADR-426, sprints HDI-00 and HDI-01).
//!
//! Opens one investigation only from an explicit intent and admits a
//! read-only [`ProbeRequest`] before any executor exists. The normalizer
//! builds a redacted snapshot bundle. Control reads that bundle and emits
//! comparable claims. Does not spawn processes or depend on taskd, the
//! shell, or the modem.

mod campaign;
mod claim;
mod control;
mod error;
mod normalize;
mod probe;
mod redact;

pub use campaign::{
    Campaign, IntentDocument, INVESTIGATE_HARDWARE, MAX_EXECUTED_PROBES, MAX_PASSES,
};
pub use claim::{discovery_gain, Claim, ClaimKind, Knowledge};
pub use control::control_from_redacted;
pub use error::ContractError;
pub use normalize::{normalize, REDACTED_FILES};
pub use probe::{ProbeName, ProbeRequest};

pub const ADR_PROBE_EXAMPLE: &str = r#"{
  "campaign_id": "uuid",
  "pass": 1,
  "device": "pci:0000:01:00.0",
  "hypothesis": "discrete GPU is bound",
  "requested_probe": "pci_slot",
  "arguments": { "slot": "0000:01:00.0" },
  "risk": "read_only"
}"#;
