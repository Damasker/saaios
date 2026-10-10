//! HDI campaign loop, fixture verifier, and scripted claims
//! (ADR-426, sprints HDI-03, HDI-04, HDI-05, and HDI-07 through HDI-15).
//!
//! A scripted investigator sees only the redacted bundle. The campaign
//! stamps the pass, checks the contract, then asks `hdi-exec` to read the
//! fixture. The sealed answer key is not a field of the investigator input.
//! This crate does not open a socket.

mod adapt;
mod intent_gate;
mod investigator;
mod loop_;
mod metrics;
mod verify;

pub use adapt::{propose_display_adaptation, AdaptationProposal, DisplayAdaptation};
pub use intent_gate::{
    Adaptation, CampaignTicket, GateDecision, HardwareIntent, HardwareVerdict, IntentAuthor,
    IntentGate, ADAPT_HARDWARE, GRAPHICS_PRECONDITION, INVESTIGATE_HARDWARE, SHIPPED_PRECONDITIONS,
};
pub use investigator::{Hunter, Investigator, Repeater};
pub use loop_::{run_campaign, CampaignError, RunReport, RunStatus};
pub use metrics::{Metrics, Transferability};
pub use verify::{verify, RecordStatus, StandVerdict, Verification, VerifyError};
