//! Attention Projection (ADR-123 ATTN-01).
//!
//! Derived view over existing Tasks and Notifications. Not a daemon,
//! not `saaios.attention`, not a second notification model.

mod model;
mod project;

pub use model::{
    AttentionActionability, AttentionItem, AttentionKey, AttentionPriority, AttentionProjection,
    AttentionRelevance, AttentionSource, AttentionSurfaces,
};
pub use project::{has_orb_attention, inbox_source_ids, project_from_entities};
