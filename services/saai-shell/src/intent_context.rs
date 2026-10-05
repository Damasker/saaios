//! Capture UI context at Intent creation. Shell does not interpret
//! language here — it only freezes what was on screen.

use crate::effective_context_space;
use crate::ContextFrameEntry;
use intent_resolution::{intent_properties, ContextSnapshot, IntentSource};
use saai_entity_protocol::Entity;
use serde_json::{Map, Value};

pub fn capture_intent_context(
    primary_space_id: &str,
    context_frame: &[ContextFrameEntry],
    focused: Option<&Entity>,
) -> ContextSnapshot {
    let effective = effective_context_space(context_frame, primary_space_id);
    ContextSnapshot::from_focus(effective, focused)
}

pub fn build_orb_intent_properties(text: &str, context: &ContextSnapshot) -> Map<String, Value> {
    intent_properties(text, IntentSource::Orb, context, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use intent_resolution::{IntentInput, CONTEXT_PROPERTY};
    use saai_entity_protocol::ObjectRef;
    use serde_json::Map;
    use uuid::Uuid;

    fn entity() -> Entity {
        let now = Utc::now();
        Entity {
            schema: 1,
            id: Uuid::from_u128(7),
            space_id: "work".into(),
            entity_type: "saaios.display".into(),
            title: "Экран".into(),
            properties: Map::new(),
            revision: 4,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn captured_focus_survives_as_object_ref_not_a_title() {
        let focused = entity();
        let context = capture_intent_context("home", &[], Some(&focused));
        let properties = build_orb_intent_properties("покажи его состояние", &context);
        assert_eq!(properties["text"], "покажи его состояние");
        assert_eq!(properties["source"], "orb");
        let stored = properties[CONTEXT_PROPERTY].clone();
        assert_eq!(stored["focused_object"]["title"], "Экран");
        assert_eq!(stored["focused_object"]["revision"], 4);
        let mut intent = focused.clone();
        intent.entity_type = "saaios.intent".into();
        intent.properties = properties;
        let parsed = IntentInput::from_entity(&intent);
        let captured = parsed.context.focused_object.expect("captured focus");
        assert_eq!(captured.object, ObjectRef::entity(Uuid::from_u128(7)));
        assert_eq!(captured.revision, 4);
    }
}
