//! Attention Projection (ADR-123 ATTN-01).
//!
//! Derived view over existing Tasks, Notifications, and one Health
//! report. Not a daemon, not `saaios.attention`.

mod model;
mod project;

pub use model::{
    one_suggested_action, AttentionActionability, AttentionContext, AttentionItem, AttentionKey,
    AttentionPriority, AttentionProjection, AttentionRelevance, AttentionSource, AttentionSurfaces,
    SuggestedAction, SuggestedActionCandidate,
};
pub use project::{
    has_orb_attention, inbox_source_ids, project_from_entities, project_with_context,
    project_with_health, project_with_suggestion,
};
pub use saai_observation::{HealthReport, HealthState};
